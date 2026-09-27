//! Git operations for Branchcraft, isolated from the desktop shell.

mod branches;
mod cloning;
mod diff;
mod history;
mod mutations;
mod remote;

use branchcraft_app_core::{
    BranchInfo, BranchOperationResult, BranchSummary, ChangeKind, CloneProgress, CommitHistory,
    CommitResult, Diff, FileStatus, GitError, GitErrorKind, RemoteInfo, RemoteOperationResult,
    RemoteProgress, RepositoryId, RepositorySnapshot,
};
use git2::{ErrorClass, ErrorCode, Repository, Status, StatusOptions};
use secrecy::SecretString;
use std::path::{Component, PathBuf};
use std::{fs, io, path::Path};

/// Boundary used by Tauri and by tests. Future Git operations extend this trait.
pub trait GitService: Send + Sync {
    /// Opens exactly the selected directory and returns its current state.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the path cannot be accessed or is not a
    /// supported Git repository.
    fn open_repository(&self, path: &Path) -> Result<RepositorySnapshot, GitError>;

    /// Re-reads an already known repository from disk.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the repository disappears, becomes
    /// inaccessible, or cannot be inspected.
    fn repository_status(&self, path: &Path) -> Result<RepositorySnapshot, GitError>;

    /// Produces a bounded diff for one changed repository-relative path.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the repository or path is invalid, the
    /// file is no longer changed, or libgit2 cannot compare the content.
    fn diff(&self, path: &Path, file_path: &str) -> Result<Diff, GitError>;

    /// Stages the current working-tree state of one changed file.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the path is invalid or the index cannot
    /// be updated.
    fn stage_file(&self, path: &Path, file_path: &str) -> Result<RepositorySnapshot, GitError>;

    /// Restores one staged file in the index to its `HEAD` state.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the path is invalid or the index cannot
    /// be updated.
    fn unstage_file(&self, path: &Path, file_path: &str) -> Result<RepositorySnapshot, GitError>;

    /// Stages every current working-tree change.
    ///
    /// # Errors
    ///
    /// Returns a structured error when repository files cannot be read or the
    /// index cannot be updated.
    fn stage_all(&self, path: &Path) -> Result<RepositorySnapshot, GitError>;

    /// Restores the complete index to `HEAD` without changing working files.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the repository or index cannot be
    /// updated.
    fn unstage_all(&self, path: &Path) -> Result<RepositorySnapshot, GitError>;

    /// Creates a commit from the current index using the repository Git identity.
    ///
    /// # Errors
    ///
    /// Returns a structured error for invalid input, missing identity, an empty
    /// index, unresolved conflicts, or a failed commit write.
    fn commit(
        &self,
        path: &Path,
        summary: &str,
        description: Option<&str>,
    ) -> Result<CommitResult, GitError>;

    /// Returns a bounded history reachable from the current `HEAD`.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the repository or commit graph cannot
    /// be read.
    fn history(&self, path: &Path) -> Result<CommitHistory, GitError>;

    /// Lists local and remote-tracking branches.
    ///
    /// # Errors
    ///
    /// Returns a structured error when repository references cannot be read.
    fn branches(&self, path: &Path) -> Result<Vec<BranchSummary>, GitError>;

    /// Creates a local branch at the current `HEAD` without checking it out.
    ///
    /// # Errors
    ///
    /// Returns a structured error for invalid or duplicate names, an unborn
    /// `HEAD`, or an in-progress Git operation.
    fn create_branch(&self, path: &Path, name: &str) -> Result<BranchOperationResult, GitError>;

    /// Safely switches the worktree to an existing local branch.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the branch is missing or local changes
    /// would be overwritten.
    fn checkout_branch(&self, path: &Path, name: &str) -> Result<BranchOperationResult, GitError>;

    /// Deletes a non-current local branch only when it is merged into `HEAD`.
    ///
    /// # Errors
    ///
    /// Returns a structured error when deletion would be unsafe or the branch
    /// cannot be updated.
    fn delete_branch(&self, path: &Path, name: &str) -> Result<BranchOperationResult, GitError>;

    /// Returns the current branch's upstream remote, preferring `origin` when
    /// no upstream is configured.
    ///
    /// # Errors
    ///
    /// Returns a structured error when repository remote configuration cannot
    /// be inspected. A repository without remotes returns `None`.
    fn primary_remote(&self, path: &Path) -> Result<Option<RemoteInfo>, GitError>;

    /// Downloads objects and updates remote-tracking references.
    ///
    /// # Errors
    ///
    /// Returns a structured remote, authentication, network, or repository
    /// error without exposing credentials or server payloads.
    fn fetch(
        &self,
        path: &Path,
        token: Option<&SecretString>,
        progress: &mut dyn FnMut(RemoteProgress),
    ) -> Result<RemoteOperationResult, GitError>;

    /// Fetches and integrates the upstream branch using fast-forward or a safe
    /// merge commit.
    ///
    /// # Errors
    ///
    /// Returns a structured error for dirty worktrees, conflicts, detached
    /// heads, missing identity, authentication, or network failures.
    fn pull(
        &self,
        path: &Path,
        token: Option<&SecretString>,
        progress: &mut dyn FnMut(RemoteProgress),
    ) -> Result<RemoteOperationResult, GitError>;

    /// Pushes the current local branch to its configured upstream.
    ///
    /// # Errors
    ///
    /// Returns a structured error for rejected/non-fast-forward updates,
    /// detached heads, authentication, or network failures.
    fn push(
        &self,
        path: &Path,
        token: Option<&SecretString>,
        progress: &mut dyn FnMut(RemoteProgress),
    ) -> Result<RemoteOperationResult, GitError>;

    /// Validates a clone source without accessing the network and identifies
    /// whether GitHub credentials may be used for it.
    ///
    /// # Errors
    ///
    /// Returns a structured error for insecure, credential-bearing, or
    /// malformed source URLs.
    fn clone_source_kind(&self, source: &str) -> Result<CloneSourceKind, GitError>;

    /// Clones into a new repository-named child of `destination_parent`.
    ///
    /// # Errors
    ///
    /// Returns a structured validation, destination, authentication, network,
    /// permission, or repository error.
    fn clone_repository(
        &self,
        source: &str,
        destination_parent: &Path,
        token: Option<&SecretString>,
        progress: &mut dyn FnMut(CloneProgress),
    ) -> Result<RepositorySnapshot, GitError>;
}

/// Credential policy for a validated clone source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneSourceKind {
    GitHubHttps,
    AnonymousHttps,
    Local,
}

/// libgit2-backed implementation of [`GitService`].
#[derive(Debug, Default, Clone, Copy)]
pub struct LibGitService;

impl LibGitService {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn snapshot(path: &Path) -> Result<RepositorySnapshot, GitError> {
        let selected_path = canonical_directory(path)?;
        let repository =
            Repository::open(&selected_path).map_err(|error| map_open_error(&error))?;
        let root = repository
            .workdir()
            .or_else(|| repository.path().parent())
            .ok_or_else(|| {
                GitError::new(
                    GitErrorKind::InvalidPath,
                    "The repository has no usable working directory.",
                    None,
                )
            })?
            .to_path_buf();
        let root = fs::canonicalize(root).map_err(|error| map_io_error(&error))?;
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("Repository")
            .to_owned();

        Ok(RepositorySnapshot {
            id: RepositoryId(root.to_string_lossy().into_owned()),
            name,
            path: root,
            branch: current_branch(&repository)?,
            files: changed_files(&repository)?,
        })
    }

    fn branch_operation_result(path: &Path) -> Result<BranchOperationResult, GitError> {
        Ok(BranchOperationResult {
            repository: Self::snapshot(path)?,
            branches: branches::list(path)?,
        })
    }
}

impl GitService for LibGitService {
    fn open_repository(&self, path: &Path) -> Result<RepositorySnapshot, GitError> {
        Self::snapshot(path)
    }

    fn repository_status(&self, path: &Path) -> Result<RepositorySnapshot, GitError> {
        Self::snapshot(path)
    }

    fn diff(&self, path: &Path, file_path: &str) -> Result<Diff, GitError> {
        diff::file_diff(path, file_path)
    }

    fn stage_file(&self, path: &Path, file_path: &str) -> Result<RepositorySnapshot, GitError> {
        mutations::stage_file(path, file_path)?;
        Self::snapshot(path)
    }

    fn unstage_file(&self, path: &Path, file_path: &str) -> Result<RepositorySnapshot, GitError> {
        mutations::unstage_file(path, file_path)?;
        Self::snapshot(path)
    }

    fn stage_all(&self, path: &Path) -> Result<RepositorySnapshot, GitError> {
        mutations::stage_all(path)?;
        Self::snapshot(path)
    }

    fn unstage_all(&self, path: &Path) -> Result<RepositorySnapshot, GitError> {
        mutations::unstage_all(path)?;
        Self::snapshot(path)
    }

    fn commit(
        &self,
        path: &Path,
        summary: &str,
        description: Option<&str>,
    ) -> Result<CommitResult, GitError> {
        let commit = mutations::commit(path, summary, description)?;
        Ok(CommitResult {
            commit,
            repository: Self::snapshot(path)?,
        })
    }

    fn history(&self, path: &Path) -> Result<CommitHistory, GitError> {
        history::history(path)
    }

    fn branches(&self, path: &Path) -> Result<Vec<BranchSummary>, GitError> {
        branches::list(path)
    }

    fn create_branch(&self, path: &Path, name: &str) -> Result<BranchOperationResult, GitError> {
        branches::create(path, name)?;
        Self::branch_operation_result(path)
    }

    fn checkout_branch(&self, path: &Path, name: &str) -> Result<BranchOperationResult, GitError> {
        branches::checkout(path, name)?;
        Self::branch_operation_result(path)
    }

    fn delete_branch(&self, path: &Path, name: &str) -> Result<BranchOperationResult, GitError> {
        branches::delete(path, name)?;
        Self::branch_operation_result(path)
    }

    fn primary_remote(&self, path: &Path) -> Result<Option<RemoteInfo>, GitError> {
        remote::primary_remote(path)
    }

    fn fetch(
        &self,
        path: &Path,
        token: Option<&SecretString>,
        progress: &mut dyn FnMut(RemoteProgress),
    ) -> Result<RemoteOperationResult, GitError> {
        remote::fetch(path, token, progress)
    }

    fn pull(
        &self,
        path: &Path,
        token: Option<&SecretString>,
        progress: &mut dyn FnMut(RemoteProgress),
    ) -> Result<RemoteOperationResult, GitError> {
        remote::pull(path, token, progress)
    }

    fn push(
        &self,
        path: &Path,
        token: Option<&SecretString>,
        progress: &mut dyn FnMut(RemoteProgress),
    ) -> Result<RemoteOperationResult, GitError> {
        remote::push(path, token, progress)
    }

    fn clone_source_kind(&self, source: &str) -> Result<CloneSourceKind, GitError> {
        cloning::source_kind(source)
    }

    fn clone_repository(
        &self,
        source: &str,
        destination_parent: &Path,
        token: Option<&SecretString>,
        progress: &mut dyn FnMut(CloneProgress),
    ) -> Result<RepositorySnapshot, GitError> {
        let destination = cloning::clone_repository(source, destination_parent, token, progress)?;
        Self::snapshot(&destination)
    }
}

pub use diff::MAX_DIFF_FILE_BYTES;

fn canonical_directory(path: &Path) -> Result<std::path::PathBuf, GitError> {
    if path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorKind::InvalidPath,
            "Choose a repository folder.",
            Some("The selected path was empty.".to_owned()),
        ));
    }

    let canonical = fs::canonicalize(path).map_err(|error| map_io_error(&error))?;
    if !canonical.is_dir() {
        return Err(GitError::new(
            GitErrorKind::InvalidPath,
            "The selected path is not a folder.",
            None,
        ));
    }
    Ok(canonical)
}

fn validate_relative_path(path: &str) -> Result<PathBuf, GitError> {
    if path.is_empty()
        || path.contains('\0')
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(GitError::new(
            GitErrorKind::InvalidPath,
            "The selected file path is invalid.",
            Some("Choose a file reported by the current repository status.".to_owned()),
        ));
    }
    Ok(PathBuf::from(path))
}

fn current_branch(repository: &Repository) -> Result<BranchInfo, GitError> {
    match repository.head() {
        Ok(head) => {
            let detached = repository.head_detached().unwrap_or(false);
            let display_name = if detached {
                head.target().map_or_else(
                    || "Detached HEAD".to_owned(),
                    |oid| oid.to_string().chars().take(8).collect(),
                )
            } else {
                head.shorthand().unwrap_or("HEAD").to_owned()
            };

            Ok(BranchInfo {
                display_name,
                detached,
                unborn: false,
            })
        }
        Err(error) if error.code() == ErrorCode::UnbornBranch => {
            let display_name = repository
                .find_reference("HEAD")
                .ok()
                .and_then(|reference| {
                    reference
                        .symbolic_target()
                        .ok()
                        .flatten()
                        .map(str::to_owned)
                })
                .and_then(|target| target.strip_prefix("refs/heads/").map(str::to_owned))
                .unwrap_or_else(|| "Unborn branch".to_owned());

            Ok(BranchInfo {
                display_name,
                detached: false,
                unborn: true,
            })
        }
        Err(error) => Err(map_git_error(&error)),
    }
}

fn changed_files(repository: &Repository) -> Result<Vec<FileStatus>, GitError> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .include_unmodified(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repository
        .statuses(Some(&mut options))
        .map_err(|error| map_git_error(&error))?;
    let mut files = statuses
        .iter()
        .filter_map(|entry| {
            let status = entry.status();
            let path = status_path(&entry)?;
            let previous_path = previous_path(&entry, status);
            Some(FileStatus {
                path,
                previous_path,
                staged: staged_kind(status),
                unstaged: unstaged_kind(status),
                conflicted: status.is_conflicted(),
            })
        })
        .collect::<Vec<_>>();
    files.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn status_path(entry: &git2::StatusEntry<'_>) -> Option<String> {
    entry
        .index_to_workdir()
        .and_then(|delta| delta.new_file().path())
        .or_else(|| {
            entry
                .head_to_index()
                .and_then(|delta| delta.new_file().path())
        })
        .or_else(|| entry.path().ok().map(Path::new))
        .map(git_path_for_ipc)
}

fn previous_path(entry: &git2::StatusEntry<'_>, status: Status) -> Option<String> {
    let old_path = if status.contains(Status::INDEX_RENAMED) {
        entry
            .head_to_index()
            .and_then(|delta| delta.old_file().path())
    } else if status.contains(Status::WT_RENAMED) {
        entry
            .index_to_workdir()
            .and_then(|delta| delta.old_file().path())
    } else {
        None
    };
    old_path.map(git_path_for_ipc)
}

fn git_path_for_ipc(path: &Path) -> String {
    #[cfg(windows)]
    {
        path.to_string_lossy().replace('\\', "/")
    }

    #[cfg(not(windows))]
    {
        path.to_string_lossy().into_owned()
    }
}

const fn staged_kind(status: Status) -> Option<ChangeKind> {
    if status.contains(Status::INDEX_NEW) {
        Some(ChangeKind::Added)
    } else if status.contains(Status::INDEX_MODIFIED) {
        Some(ChangeKind::Modified)
    } else if status.contains(Status::INDEX_DELETED) {
        Some(ChangeKind::Deleted)
    } else if status.contains(Status::INDEX_RENAMED) {
        Some(ChangeKind::Renamed)
    } else if status.contains(Status::INDEX_TYPECHANGE) {
        Some(ChangeKind::TypeChanged)
    } else {
        None
    }
}

const fn unstaged_kind(status: Status) -> Option<ChangeKind> {
    if status.contains(Status::WT_NEW) {
        Some(ChangeKind::Untracked)
    } else if status.contains(Status::WT_MODIFIED) {
        Some(ChangeKind::Modified)
    } else if status.contains(Status::WT_DELETED) {
        Some(ChangeKind::Deleted)
    } else if status.contains(Status::WT_RENAMED) {
        Some(ChangeKind::Renamed)
    } else if status.contains(Status::WT_TYPECHANGE) {
        Some(ChangeKind::TypeChanged)
    } else {
        None
    }
}

fn map_io_error(error: &io::Error) -> GitError {
    let (kind, summary) = match error.kind() {
        io::ErrorKind::NotFound => (
            GitErrorKind::RepositoryNotFound,
            "The selected folder no longer exists.",
        ),
        io::ErrorKind::PermissionDenied => (
            GitErrorKind::PermissionDenied,
            "Branchcraft does not have permission to open this folder.",
        ),
        _ => (
            GitErrorKind::InvalidPath,
            "Branchcraft could not open the selected folder.",
        ),
    };
    GitError::new(kind, summary, None)
}

fn map_git_error(error: &git2::Error) -> GitError {
    let (kind, summary, detail) =
        if error.class() == ErrorClass::Repository || error.code() == ErrorCode::NotFound {
            (
                GitErrorKind::NotRepository,
                "This folder is not a Git repository.",
                Some("Choose a folder that contains a .git directory.".to_owned()),
            )
        } else if error.code() == ErrorCode::Auth {
            (
                GitErrorKind::PermissionDenied,
                "Branchcraft could not read this repository.",
                Some("Check the folder permissions and try again.".to_owned()),
            )
        } else {
            (
                GitErrorKind::Unknown,
                "Branchcraft could not inspect this repository.",
                Some("The repository may be damaged or use an unsupported format.".to_owned()),
            )
        };
    GitError::new(kind, summary, detail)
}

fn map_open_error(error: &git2::Error) -> GitError {
    if error.code() == ErrorCode::Auth {
        GitError::new(
            GitErrorKind::PermissionDenied,
            "Branchcraft could not read this repository.",
            Some("Check the folder permissions and try again.".to_owned()),
        )
    } else {
        GitError::new(
            GitErrorKind::NotRepository,
            "This folder is not a Git repository.",
            Some("Choose a folder that contains a .git directory.".to_owned()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{GitService, LibGitService};
    use branchcraft_app_core::{ChangeKind, GitErrorKind};
    use branchcraft_test_support::TestRepository;

    #[test]
    fn rejects_a_non_repository_directory() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let error = LibGitService::new()
            .open_repository(directory.path())
            .expect_err("plain directory must be rejected");

        assert_eq!(error.kind, GitErrorKind::NotRepository);
    }

    #[test]
    fn reports_repository_identity_and_clean_status() {
        let repository = TestRepository::with_initial_commit();
        let snapshot = LibGitService::new()
            .open_repository(repository.path())
            .expect("open repository");

        assert_eq!(snapshot.path, repository.path().canonicalize().unwrap());
        assert!(!snapshot.branch.detached);
        assert!(!snapshot.branch.unborn);
        assert!(snapshot.files.is_empty());
    }

    #[test]
    fn reports_modified_untracked_and_deleted_files() {
        let repository = TestRepository::with_initial_commit();
        repository.write("README.md", "changed\n");
        repository.write("src/new.rs", "fn new_file() {}\n");
        repository.remove("delete-me.txt");

        let snapshot = LibGitService::new()
            .repository_status(repository.path())
            .expect("repository status");

        let readme = snapshot
            .files
            .iter()
            .find(|file| file.path == "README.md")
            .unwrap();
        assert_eq!(readme.unstaged, Some(ChangeKind::Modified));

        let added = snapshot
            .files
            .iter()
            .find(|file| file.path == "src/new.rs")
            .unwrap();
        assert_eq!(added.unstaged, Some(ChangeKind::Untracked));

        let deleted = snapshot
            .files
            .iter()
            .find(|file| file.path == "delete-me.txt")
            .unwrap();
        assert_eq!(deleted.unstaged, Some(ChangeKind::Deleted));
    }

    #[test]
    fn keeps_staged_and_unstaged_states_separate() {
        let repository = TestRepository::with_initial_commit();
        repository.write("README.md", "staged\n");
        repository.stage("README.md");
        repository.write("README.md", "staged and then changed again\n");

        let snapshot = LibGitService::new()
            .repository_status(repository.path())
            .expect("repository status");
        let readme = snapshot
            .files
            .iter()
            .find(|file| file.path == "README.md")
            .unwrap();

        assert_eq!(readme.staged, Some(ChangeKind::Modified));
        assert_eq!(readme.unstaged, Some(ChangeKind::Modified));
    }

    #[test]
    fn reports_an_unborn_branch_for_empty_repositories() {
        let repository = TestRepository::empty();
        let snapshot = LibGitService::new()
            .open_repository(repository.path())
            .expect("open empty repository");

        assert!(snapshot.branch.unborn);
        assert!(snapshot.files.is_empty());
    }

    #[test]
    fn reports_staged_renames_with_the_previous_path() {
        let repository = TestRepository::with_initial_commit();
        repository.rename_and_stage("delete-me.txt", "archive/kept.txt");

        let snapshot = LibGitService::new()
            .repository_status(repository.path())
            .expect("repository status");
        let renamed = snapshot
            .files
            .iter()
            .find(|file| file.path == "archive/kept.txt")
            .expect("renamed file");

        assert_eq!(renamed.staged, Some(ChangeKind::Renamed));
        assert_eq!(renamed.previous_path.as_deref(), Some("delete-me.txt"));
    }

    #[test]
    fn reports_detached_head_as_a_short_commit_id() {
        let repository = TestRepository::with_initial_commit();
        repository.detach_head();

        let snapshot = LibGitService::new()
            .repository_status(repository.path())
            .expect("repository status");

        assert!(snapshot.branch.detached);
        assert!(!snapshot.branch.unborn);
        assert_eq!(snapshot.branch.display_name.len(), 8);
    }

    #[test]
    fn handles_portable_repository_and_file_paths() {
        let repository = TestRepository::with_initial_commit_and_prefix("Branchcraft space Гілка-");
        let relative_path = concat!(
            "Mixed Case/level-01/level-02/level-03/level-04/level-05/",
            "level-06/level-07/résumé + [draft]!.txt"
        );
        repository.write(relative_path, "portable path\n");

        let service = LibGitService::new();
        let snapshot = service
            .open_repository(repository.path())
            .expect("open portable repository path");
        assert_eq!(snapshot.path, repository.path().canonicalize().unwrap());
        #[cfg(windows)]
        assert!(matches!(
            snapshot.path.components().next(),
            Some(std::path::Component::Prefix(_))
        ));
        let file = snapshot
            .files
            .iter()
            .find(|file| file.path == relative_path)
            .expect("portable status path");
        assert_eq!(file.unstaged, Some(ChangeKind::Untracked));

        let staged = service
            .stage_file(repository.path(), relative_path)
            .expect("stage portable path");
        assert_eq!(
            staged
                .files
                .iter()
                .find(|file| file.path == relative_path)
                .and_then(|file| file.staged),
            Some(ChangeKind::Added)
        );
    }
}
