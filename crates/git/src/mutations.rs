use super::{canonical_directory, changed_files, map_open_error, validate_relative_path};
use branchcraft_app_core::{CommitInfo, FileStatus, GitError, GitErrorKind};
use branchcraft_hooks::{GitHook, HookError, HookErrorKind, HookEvent, HookRunner};
use git2::{ErrorCode, Index, ObjectType, Repository};
use std::{fs, io, path::Path};

const MAX_COMMIT_MESSAGE_BYTES: u64 = 1024 * 1024;

pub(super) fn stage_file(repository_path: &Path, file_path: &str) -> Result<(), GitError> {
    let repository = open_repository(repository_path)?;
    let relative_path = validate_relative_path(file_path)?;
    let status = find_status(&repository, file_path)?;
    let mut index = repository.index().map_err(|error| map_git_error(&error))?;

    stage_status(&repository, &mut index, &status, &relative_path)?;
    index.write().map_err(|error| map_git_error(&error))
}

pub(super) fn unstage_file(repository_path: &Path, file_path: &str) -> Result<(), GitError> {
    let repository = open_repository(repository_path)?;
    let relative_path = validate_relative_path(file_path)?;
    let status = find_status(&repository, file_path)?;

    if status.staged.is_none() && !status.conflicted {
        return Ok(());
    }

    let mut paths = vec![relative_path];
    if let Some(previous_path) = status.previous_path {
        paths.push(validate_relative_path(&previous_path)?);
    }
    let head = head_object(&repository)?;
    repository
        .reset_default(head.as_ref(), &paths)
        .map_err(|error| map_git_error(&error))
}

pub(super) fn stage_all(repository_path: &Path) -> Result<(), GitError> {
    let repository = open_repository(repository_path)?;
    let statuses = changed_files(&repository)?;
    let mut index = repository.index().map_err(|error| map_git_error(&error))?;

    for status in statuses {
        if status.unstaged.is_some() || status.conflicted {
            let relative_path = validate_relative_path(&status.path)?;
            stage_status(&repository, &mut index, &status, &relative_path)?;
        }
    }
    index.write().map_err(|error| map_git_error(&error))
}

pub(super) fn unstage_all(repository_path: &Path) -> Result<(), GitError> {
    let repository = open_repository(repository_path)?;
    let statuses = changed_files(&repository)?;
    let paths = statuses
        .iter()
        .filter(|status| status.staged.is_some() || status.conflicted)
        .flat_map(|status| {
            [Some(status.path.as_str()), status.previous_path.as_deref()]
                .into_iter()
                .flatten()
        })
        .map(validate_relative_path)
        .collect::<Result<Vec<_>, _>>()?;

    if paths.is_empty() {
        return Ok(());
    }
    let head = head_object(&repository)?;
    repository
        .reset_default(head.as_ref(), &paths)
        .map_err(|error| map_git_error(&error))
}

pub(super) fn commit(
    repository_path: &Path,
    summary: &str,
    description: Option<&str>,
    on_hook_event: &mut dyn FnMut(HookEvent),
) -> Result<CommitInfo, GitError> {
    let summary = summary.trim();
    if summary.is_empty() {
        return Err(GitError::new(
            GitErrorKind::InvalidInput,
            "Enter a commit summary.",
            Some("The summary is required before a commit can be created.".to_owned()),
        ));
    }
    if summary.chars().count() > 120
        || summary.contains('\r')
        || summary.contains('\n')
        || summary.contains('\0')
    {
        return Err(GitError::new(
            GitErrorKind::InvalidInput,
            "The commit summary is not valid.",
            Some("Use one line containing no more than 120 characters.".to_owned()),
        ));
    }
    let description = description.map(str::trim).filter(|value| !value.is_empty());
    if description.is_some_and(|body| body.chars().count() > 10_000 || body.contains('\0')) {
        return Err(GitError::new(
            GitErrorKind::InvalidInput,
            "The commit description is not valid.",
            Some("Descriptions can contain up to 10,000 characters.".to_owned()),
        ));
    }

    let repository = open_repository(repository_path)?;
    let worktree = repository
        .workdir()
        .ok_or_else(unsupported_bare_repository)?;
    validate_commit_index(&repository)?;
    let signature = repository.signature().map_err(|_| {
        GitError::new(
            GitErrorKind::MissingIdentity,
            "Git needs your name and email before committing.",
            Some(
                "Configure user.name and user.email for this repository or in your global Git configuration."
                    .to_owned(),
            ),
        )
    })?;

    let configured_hooks_path = repository
        .config()
        .ok()
        .and_then(|config| config.get_path("core.hooksPath").ok());
    let hooks = HookRunner::new(
        worktree,
        repository.path(),
        configured_hooks_path.as_deref(),
    );
    hooks
        .run(GitHook::PreCommit, &[], on_hook_event)
        .map_err(|error| map_hook_error(&error))?;

    let message =
        description.map_or_else(|| summary.to_owned(), |body| format!("{summary}\n\n{body}"));
    let message_path = repository.path().join("COMMIT_EDITMSG");
    fs::write(&message_path, message).map_err(|error| commit_message_io_error(&error))?;
    hooks
        .run(
            GitHook::PrepareCommitMsg,
            &[message_path.as_path(), Path::new("message")],
            on_hook_event,
        )
        .map_err(|error| map_hook_error(&error))?;
    hooks
        .run(GitHook::CommitMsg, &[message_path.as_path()], on_hook_event)
        .map_err(|error| map_hook_error(&error))?;
    let (message, committed_summary) = read_hook_commit_message(&message_path)?;

    let mut index = validate_commit_index(&repository)?;
    let tree_id = index.write_tree().map_err(|error| map_git_error(&error))?;
    let tree = repository
        .find_tree(tree_id)
        .map_err(|error| map_git_error(&error))?;
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let parents = parent.iter().collect::<Vec<_>>();
    let oid = repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &parents,
        )
        .map_err(|error| map_git_error(&error))?;
    let id = oid.to_string();

    // Git treats post-commit as informational: its failure cannot undo the
    // commit that was already written. The failure event remains visible in
    // the hook terminal for the user to diagnose.
    let _ = hooks.run(GitHook::PostCommit, &[], on_hook_event);

    Ok(CommitInfo {
        short_id: id.chars().take(8).collect(),
        id,
        summary: committed_summary,
    })
}

fn validate_commit_index(repository: &Repository) -> Result<Index, GitError> {
    let index = repository.index().map_err(|error| map_git_error(&error))?;
    if index.has_conflicts() {
        return Err(GitError::new(
            GitErrorKind::Conflict,
            "Resolve conflicts before committing.",
            Some("The repository index still contains unresolved files.".to_owned()),
        ));
    }
    ensure_staged_changes(repository, &index)?;
    Ok(index)
}

fn read_hook_commit_message(path: &Path) -> Result<(String, String), GitError> {
    let metadata = path
        .metadata()
        .map_err(|error| commit_message_io_error(&error))?;
    if metadata.len() > MAX_COMMIT_MESSAGE_BYTES {
        return Err(GitError::new(
            GitErrorKind::HookFailed,
            "A Git hook produced an oversized commit message.",
            Some("Keep the commit message below 1 MiB and try again.".to_owned()),
        ));
    }
    let message = fs::read_to_string(path).map_err(|error| commit_message_io_error(&error))?;
    if message.contains('\0') {
        return Err(GitError::new(
            GitErrorKind::HookFailed,
            "A Git hook produced an invalid commit message.",
            Some("Commit messages cannot contain null bytes.".to_owned()),
        ));
    }
    let summary = message.lines().next().map(str::trim).unwrap_or_default();
    if summary.is_empty() {
        return Err(GitError::new(
            GitErrorKind::HookFailed,
            "A Git hook removed the commit summary.",
            Some("Update the hook so it leaves a non-empty first line.".to_owned()),
        ));
    }
    let summary = summary.to_owned();
    Ok((message, summary))
}

fn map_hook_error(error: &HookError) -> GitError {
    let detail = match error.kind {
        HookErrorKind::Rejected => error.exit_code.map_or_else(
            || "Review the hook output, then try the commit again.".to_owned(),
            |code| format!("The hook exited with code {code}. Review its output and try again."),
        ),
        HookErrorKind::Start => {
            "Check that the hook is executable and its interpreter is installed.".to_owned()
        }
        HookErrorKind::Output => {
            "Branchcraft could not capture all output from the hook.".to_owned()
        }
    };
    GitError::new(
        GitErrorKind::HookFailed,
        format!("The {} hook stopped the commit.", error.hook.display_name()),
        Some(detail),
    )
}

fn commit_message_io_error(error: &io::Error) -> GitError {
    let kind = if error.kind() == io::ErrorKind::PermissionDenied {
        GitErrorKind::PermissionDenied
    } else {
        GitErrorKind::HookFailed
    };
    GitError::new(
        kind,
        "Branchcraft could not prepare the commit message for Git hooks.",
        Some("Check the repository metadata permissions and try again.".to_owned()),
    )
}

fn open_repository(path: &Path) -> Result<Repository, GitError> {
    let selected_path = canonical_directory(path)?;
    Repository::open(selected_path).map_err(|error| map_open_error(&error))
}

fn find_status(repository: &Repository, file_path: &str) -> Result<FileStatus, GitError> {
    changed_files(repository)?
        .into_iter()
        .find(|status| status.path == file_path)
        .ok_or_else(|| {
            GitError::new(
                GitErrorKind::InvalidPath,
                "This file is no longer changed.",
                Some("Refresh the repository to update the changes list.".to_owned()),
            )
        })
}

fn stage_status(
    repository: &Repository,
    index: &mut Index,
    status: &FileStatus,
    relative_path: &Path,
) -> Result<(), GitError> {
    if let Some(previous_path) = status.previous_path.as_deref() {
        let previous_path = validate_relative_path(previous_path)?;
        if index.get_path(&previous_path, 0).is_some() {
            index
                .remove_path(&previous_path)
                .map_err(|error| map_git_error(&error))?;
        }
    }

    let workdir = repository
        .workdir()
        .ok_or_else(unsupported_bare_repository)?;
    match workdir.join(relative_path).symlink_metadata() {
        Ok(_) => index
            .add_path(relative_path)
            .map_err(|error| map_git_error(&error))?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if index.get_path(relative_path, 0).is_some() || status.conflicted {
                index
                    .remove_path(relative_path)
                    .map_err(|error| map_git_error(&error))?;
            }
        }
        Err(error) => return Err(map_io_error(&error)),
    }
    Ok(())
}

fn head_object(repository: &Repository) -> Result<Option<git2::Object<'_>>, GitError> {
    match repository.head() {
        Ok(head) => head
            .peel(ObjectType::Commit)
            .map(Some)
            .map_err(|error| map_git_error(&error)),
        Err(error) if error.code() == ErrorCode::UnbornBranch => Ok(None),
        Err(error) => Err(map_git_error(&error)),
    }
}

fn ensure_staged_changes(repository: &Repository, index: &Index) -> Result<(), GitError> {
    let head_tree = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_tree().ok());
    let diff = repository
        .diff_tree_to_index(head_tree.as_ref(), Some(index), None)
        .map_err(|error| map_git_error(&error))?;
    if diff.deltas().len() == 0 {
        Err(GitError::new(
            GitErrorKind::NothingStaged,
            "There are no staged changes to commit.",
            Some("Stage at least one changed file and try again.".to_owned()),
        ))
    } else {
        Ok(())
    }
}

fn unsupported_bare_repository() -> GitError {
    GitError::new(
        GitErrorKind::InvalidPath,
        "This repository has no working directory.",
        None,
    )
}

fn map_git_error(error: &git2::Error) -> GitError {
    let (kind, summary, detail) = match error.code() {
        ErrorCode::Auth => (
            GitErrorKind::PermissionDenied,
            "Branchcraft could not update this repository.",
            Some("Check the repository permissions and try again.".to_owned()),
        ),
        ErrorCode::Conflict | ErrorCode::Locked => (
            GitErrorKind::Conflict,
            "Git could not update the repository index.",
            Some("Another Git operation or an unresolved conflict may be in progress.".to_owned()),
        ),
        _ => (
            GitErrorKind::Unknown,
            "Branchcraft could not update this repository.",
            Some("The repository may have changed during the operation.".to_owned()),
        ),
    };
    GitError::new(kind, summary, detail)
}

fn map_io_error(error: &io::Error) -> GitError {
    let kind = if error.kind() == io::ErrorKind::PermissionDenied {
        GitErrorKind::PermissionDenied
    } else {
        GitErrorKind::Unknown
    };
    GitError::new(
        kind,
        "Branchcraft could not read this working-tree file.",
        Some("The file may have moved or its permissions may have changed.".to_owned()),
    )
}

#[cfg(test)]
mod tests {
    use crate::{GitService, LibGitService};
    use branchcraft_app_core::{ChangeKind, GitErrorKind};
    use branchcraft_hooks::{GitHook, HookEventKind};
    use branchcraft_test_support::TestRepository;
    use std::fs;

    fn write_hook(repository: &TestRepository, name: &str, body: &str) {
        let path = repository.path().join(".git").join("hooks").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = path.metadata().expect("hook metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("hook permissions");
        }
    }

    #[test]
    fn stages_and_unstages_one_file_without_changing_the_worktree() {
        let repository = TestRepository::with_initial_commit();
        repository.write("README.md", "changed\n");
        let service = LibGitService::new();

        let staged = service
            .stage_file(repository.path(), "README.md")
            .expect("stage file");
        let file = staged
            .files
            .iter()
            .find(|file| file.path == "README.md")
            .unwrap();
        assert_eq!(file.staged, Some(ChangeKind::Modified));
        assert_eq!(file.unstaged, None);

        let unstaged = service
            .unstage_file(repository.path(), "README.md")
            .expect("unstage file");
        let file = unstaged
            .files
            .iter()
            .find(|file| file.path == "README.md")
            .unwrap();
        assert_eq!(file.staged, None);
        assert_eq!(file.unstaged, Some(ChangeKind::Modified));
        assert_eq!(repository.read("README.md"), "changed\n");
    }

    #[test]
    fn stages_and_unstages_all_change_kinds() {
        let repository = TestRepository::with_initial_commit();
        repository.write("README.md", "changed\n");
        repository.write("new.txt", "new\n");
        repository.remove("delete-me.txt");
        let service = LibGitService::new();

        let staged = service.stage_all(repository.path()).expect("stage all");
        assert!(staged.files.iter().all(|file| file.staged.is_some()));
        assert!(staged.files.iter().all(|file| file.unstaged.is_none()));

        let unstaged = service.unstage_all(repository.path()).expect("unstage all");
        assert!(unstaged.files.iter().all(|file| file.staged.is_none()));
        assert!(unstaged.files.iter().all(|file| file.unstaged.is_some()));
    }

    #[test]
    fn unstages_both_sides_of_a_rename() {
        let repository = TestRepository::with_initial_commit();
        repository.rename_and_stage("delete-me.txt", "archive/kept.txt");

        let snapshot = LibGitService::new()
            .unstage_file(repository.path(), "archive/kept.txt")
            .expect("unstage rename");

        assert!(snapshot.files.iter().all(|file| file.staged.is_none()));
        let renamed = snapshot
            .files
            .iter()
            .find(|file| file.path == "archive/kept.txt")
            .expect("unstaged rename remains");
        assert_eq!(renamed.unstaged, Some(ChangeKind::Renamed));
        assert_eq!(renamed.previous_path.as_deref(), Some("delete-me.txt"));
    }

    #[test]
    fn creates_a_commit_with_summary_and_description() {
        let repository = TestRepository::with_initial_commit();
        repository.configure_identity("Branchcraft User", "user@branchcraft.local");
        repository.write("README.md", "committed\n");
        let service = LibGitService::new();
        service
            .stage_file(repository.path(), "README.md")
            .expect("stage file");

        let result = service
            .commit(
                repository.path(),
                "Update README",
                Some("Explain the local change."),
                &mut |_| {},
            )
            .expect("create commit");

        assert_eq!(result.commit.summary, "Update README");
        assert_eq!(result.commit.short_id.len(), 8);
        assert!(result.repository.files.is_empty());
        assert_eq!(
            repository.head_message(),
            "Update README\n\nExplain the local change."
        );
    }

    #[test]
    fn commit_msg_hook_can_update_the_message_and_stream_output() {
        let repository = TestRepository::with_initial_commit();
        repository.configure_identity("Branchcraft User", "user@branchcraft.local");
        repository.write("README.md", "hooked\n");
        repository.stage("README.md");
        write_hook(
            &repository,
            "commit-msg",
            "printf 'Hooked summary\\n\\nChanged by hook.\\n' > \"$1\"\nprintf 'message updated\\n'",
        );
        let mut events = Vec::new();

        let result = LibGitService::new()
            .commit(repository.path(), "Original summary", None, &mut |event| {
                events.push(event);
            })
            .expect("hooked commit");

        assert_eq!(result.commit.summary, "Hooked summary");
        assert_eq!(
            repository.head_message(),
            "Hooked summary\n\nChanged by hook.\n"
        );
        assert!(events.iter().any(|event| {
            event.hook == GitHook::CommitMsg && event.kind == HookEventKind::Stdout
        }));
    }

    #[test]
    fn rejecting_pre_commit_hook_prevents_the_commit() {
        let repository = TestRepository::with_initial_commit();
        repository.configure_identity("Branchcraft User", "user@branchcraft.local");
        repository.write("README.md", "rejected\n");
        repository.stage("README.md");
        write_hook(
            &repository,
            "pre-commit",
            "printf 'quality gate failed\\n' >&2\nexit 9",
        );
        let mut events = Vec::new();

        let error = LibGitService::new()
            .commit(repository.path(), "Rejected commit", None, &mut |event| {
                events.push(event);
            })
            .expect_err("pre-commit rejection");

        assert_eq!(error.kind, GitErrorKind::HookFailed);
        assert_eq!(repository.head_message(), "Initial commit");
        assert_eq!(
            events.last().map(|event| (event.hook, event.kind)),
            Some((GitHook::PreCommit, HookEventKind::Failed))
        );
    }

    #[test]
    fn failing_post_commit_hook_does_not_undo_the_commit() {
        let repository = TestRepository::with_initial_commit();
        repository.configure_identity("Branchcraft User", "user@branchcraft.local");
        repository.write("README.md", "committed\n");
        repository.stage("README.md");
        write_hook(
            &repository,
            "post-commit",
            "printf 'notify failed\\n' >&2\nexit 5",
        );
        let mut events = Vec::new();

        let result = LibGitService::new()
            .commit(
                repository.path(),
                "Committed despite notification",
                None,
                &mut |event| events.push(event),
            )
            .expect("post-commit cannot reject commit");

        assert_eq!(result.commit.summary, "Committed despite notification");
        assert_eq!(
            events.last().map(|event| (event.hook, event.kind)),
            Some((GitHook::PostCommit, HookEventKind::Failed))
        );
    }

    #[test]
    fn commit_preserves_unstaged_changes() {
        let repository = TestRepository::with_initial_commit();
        repository.configure_identity("Branchcraft User", "user@branchcraft.local");
        repository.write("README.md", "staged\n");
        repository.stage("README.md");
        repository.write("README.md", "still unstaged\n");

        let result = LibGitService::new()
            .commit(
                repository.path(),
                "Commit staged content",
                None,
                &mut |_| {},
            )
            .expect("create commit");
        let file = result
            .repository
            .files
            .iter()
            .find(|file| file.path == "README.md")
            .expect("unstaged file remains");

        assert_eq!(file.staged, None);
        assert_eq!(file.unstaged, Some(ChangeKind::Modified));
        assert_eq!(repository.read("README.md"), "still unstaged\n");
    }

    #[test]
    fn creates_the_first_commit_on_an_unborn_branch() {
        let repository = TestRepository::empty();
        repository.configure_identity("Branchcraft User", "user@branchcraft.local");
        repository.write("README.md", "first\n");
        let service = LibGitService::new();
        service
            .stage_all(repository.path())
            .expect("stage initial file");

        let result = service
            .commit(repository.path(), "Initial commit", None, &mut |_| {})
            .expect("create initial commit");

        assert_eq!(result.commit.summary, "Initial commit");
        assert!(!result.repository.branch.unborn);
        assert!(result.repository.files.is_empty());
    }

    #[test]
    fn rejects_empty_messages_nothing_staged_and_missing_identity() {
        let repository = TestRepository::with_initial_commit();
        let service = LibGitService::new();

        let empty = service
            .commit(repository.path(), "   ", None, &mut |_| {})
            .expect_err("empty summary must fail");
        assert_eq!(empty.kind, GitErrorKind::InvalidInput);

        let multiline = service
            .commit(repository.path(), "summary\nbody", None, &mut |_| {})
            .expect_err("multiline summary must fail");
        assert_eq!(multiline.kind, GitErrorKind::InvalidInput);

        repository.configure_identity("Branchcraft User", "user@branchcraft.local");
        let nothing = service
            .commit(repository.path(), "No changes", None, &mut |_| {})
            .expect_err("nothing staged must fail");
        assert_eq!(nothing.kind, GitErrorKind::NothingStaged);

        repository.write("README.md", "changed\n");
        repository.stage("README.md");
        repository.clear_identity();
        let identity = service
            .commit(repository.path(), "Needs identity", None, &mut |_| {})
            .expect_err("missing identity must fail");
        assert_eq!(identity.kind, GitErrorKind::MissingIdentity);
    }

    #[test]
    fn mutation_paths_cannot_escape_the_repository() {
        let repository = TestRepository::with_initial_commit();
        let error = LibGitService::new()
            .stage_file(repository.path(), "../outside.txt")
            .expect_err("path traversal must fail");

        assert_eq!(error.kind, GitErrorKind::InvalidPath);
    }
}
