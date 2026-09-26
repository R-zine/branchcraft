use super::{canonical_directory, map_open_error};
use branchcraft_app_core::{BranchKind, BranchSummary, GitError, GitErrorKind};
use git2::{Branch, BranchType, ErrorCode, Repository, RepositoryState, build::CheckoutBuilder};
use std::path::Path;

pub(super) fn list(repository_path: &Path) -> Result<Vec<BranchSummary>, GitError> {
    let repository = open_repository(repository_path)?;
    list_repository(&repository)
}

pub(super) fn create(repository_path: &Path, name: &str) -> Result<(), GitError> {
    let repository = open_repository(repository_path)?;
    ensure_clean_operation_state(&repository)?;
    let name = validate_branch_name(name)?;
    let target = repository
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|error| {
            if error.code() == ErrorCode::UnbornBranch {
                GitError::new(
                    GitErrorKind::InvalidReference,
                    "Create the first commit before creating another branch.",
                    None,
                )
            } else {
                map_error(&error, "create this branch")
            }
        })?;
    repository
        .branch(name, &target, false)
        .map_err(|error| map_error(&error, "create this branch"))?;
    Ok(())
}

pub(super) fn checkout(repository_path: &Path, name: &str) -> Result<(), GitError> {
    let repository = open_repository(repository_path)?;
    ensure_clean_operation_state(&repository)?;
    let name = validate_branch_name(name)?;
    let branch = repository
        .find_branch(name, BranchType::Local)
        .map_err(|error| map_error(&error, "switch branches"))?;
    if branch.is_head() {
        return Ok(());
    }
    let reference_name = branch
        .get()
        .name()
        .map_err(|error| map_error(&error, "read this branch reference"))?;
    let target = branch
        .get()
        .peel_to_commit()
        .map_err(|error| map_error(&error, "switch branches"))?;
    let mut checkout = CheckoutBuilder::new();
    checkout.safe();
    repository
        .checkout_tree(target.as_object(), Some(&mut checkout))
        .map_err(|error| map_checkout_error(&error))?;
    repository
        .set_head(reference_name)
        .map_err(|error| map_error(&error, "switch branches"))
}

pub(super) fn delete(repository_path: &Path, name: &str) -> Result<(), GitError> {
    let repository = open_repository(repository_path)?;
    ensure_clean_operation_state(&repository)?;
    let name = validate_branch_name(name)?;
    let mut branch = repository
        .find_branch(name, BranchType::Local)
        .map_err(|error| map_error(&error, "delete this branch"))?;
    if branch.is_head() {
        return Err(GitError::new(
            GitErrorKind::UnsafeOperation,
            "The current branch cannot be deleted.",
            Some("Switch to a different branch first.".to_owned()),
        ));
    }

    let branch_id = branch
        .get()
        .peel_to_commit()
        .map_err(|error| map_error(&error, "delete this branch"))?
        .id();
    let head_id = repository
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|error| map_error(&error, "delete this branch"))?
        .id();
    let merged = branch_id == head_id
        || repository
            .graph_descendant_of(head_id, branch_id)
            .map_err(|error| map_error(&error, "check whether this branch is merged"))?;
    if !merged {
        return Err(GitError::new(
            GitErrorKind::UnsafeOperation,
            "This branch has commits that are not merged into HEAD.",
            Some("Branchcraft only performs safe branch deletion.".to_owned()),
        ));
    }

    branch
        .delete()
        .map_err(|error| map_error(&error, "delete this branch"))
}

fn list_repository(repository: &Repository) -> Result<Vec<BranchSummary>, GitError> {
    let mut branches = repository
        .branches(None)
        .map_err(|error| map_error(&error, "list branches"))?
        .map(|item| {
            let (branch, branch_type) = item.map_err(|error| map_error(&error, "list branches"))?;
            branch_summary(&branch, branch_type)
        })
        .collect::<Result<Vec<_>, GitError>>()?;
    branches.sort_unstable_by(|left, right| {
        right
            .current
            .cmp(&left.current)
            .then_with(|| branch_kind_order(left.kind).cmp(&branch_kind_order(right.kind)))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(branches)
}

fn branch_summary(branch: &Branch<'_>, branch_type: BranchType) -> Result<BranchSummary, GitError> {
    let name = String::from_utf8_lossy(
        branch
            .name_bytes()
            .map_err(|error| map_error(&error, "read a branch name"))?,
    )
    .into_owned();
    let id = branch
        .get()
        .peel_to_commit()
        .map_err(|error| map_error(&error, "read a branch target"))?
        .id()
        .to_string();
    let kind = match branch_type {
        BranchType::Local => BranchKind::Local,
        BranchType::Remote => BranchKind::Remote,
    };
    let upstream = (branch_type == BranchType::Local)
        .then(|| branch.upstream().ok())
        .flatten()
        .and_then(|upstream| upstream.name_bytes().ok().map(ToOwned::to_owned))
        .map(|name| String::from_utf8_lossy(&name).into_owned());

    Ok(BranchSummary {
        name,
        kind,
        current: branch.is_head(),
        short_target: id.chars().take(8).collect(),
        target: id,
        upstream,
    })
}

const fn branch_kind_order(kind: BranchKind) -> u8 {
    match kind {
        BranchKind::Local => 0,
        BranchKind::Remote => 1,
    }
}

fn open_repository(path: &Path) -> Result<Repository, GitError> {
    let selected_path = canonical_directory(path)?;
    Repository::open(selected_path).map_err(|error| map_open_error(&error))
}

fn validate_branch_name(name: &str) -> Result<&str, GitError> {
    let name = name.trim();
    if name.is_empty() || !Branch::name_is_valid(name).unwrap_or(false) {
        return Err(GitError::new(
            GitErrorKind::InvalidReference,
            "Enter a valid branch name.",
            Some(
                "Branch names cannot contain spaces, consecutive dots, or Git control characters."
                    .to_owned(),
            ),
        ));
    }
    Ok(name)
}

fn ensure_clean_operation_state(repository: &Repository) -> Result<(), GitError> {
    if repository.state() == RepositoryState::Clean {
        Ok(())
    } else {
        Err(GitError::new(
            GitErrorKind::Conflict,
            "Finish the current Git operation before managing branches.",
            Some("A merge, rebase, cherry-pick, or revert is still in progress.".to_owned()),
        ))
    }
}

fn map_checkout_error(error: &git2::Error) -> GitError {
    if matches!(
        error.code(),
        ErrorCode::Conflict | ErrorCode::Uncommitted | ErrorCode::IndexDirty
    ) {
        GitError::new(
            GitErrorKind::DirtyWorkingTree,
            "Local changes would be overwritten by switching branches.",
            Some("Commit or move those changes, then try again.".to_owned()),
        )
    } else {
        map_error(error, "switch branches")
    }
}

fn map_error(error: &git2::Error, operation: &str) -> GitError {
    let (kind, summary, detail) = match error.code() {
        ErrorCode::Exists => (
            GitErrorKind::BranchAlreadyExists,
            "A branch with this name already exists.",
            None,
        ),
        ErrorCode::NotFound => (
            GitErrorKind::BranchNotFound,
            "This branch no longer exists.",
            Some("Refresh the branch list and try again.".to_owned()),
        ),
        ErrorCode::InvalidSpec | ErrorCode::Invalid => (
            GitErrorKind::InvalidReference,
            "The branch name or reference is invalid.",
            None,
        ),
        ErrorCode::Conflict | ErrorCode::Uncommitted | ErrorCode::IndexDirty => (
            GitErrorKind::DirtyWorkingTree,
            "Local changes prevent this branch operation.",
            Some("Commit or move those changes, then try again.".to_owned()),
        ),
        ErrorCode::Locked | ErrorCode::Unmerged | ErrorCode::MergeConflict => (
            GitErrorKind::Conflict,
            "Another Git operation prevents branch changes.",
            Some("Finish the current Git operation and try again.".to_owned()),
        ),
        _ => (
            GitErrorKind::Unknown,
            "Branchcraft could not complete the branch operation.",
            Some(format!("Git could not {operation}.")),
        ),
    };
    GitError::new(kind, summary, detail)
}

#[cfg(test)]
mod tests {
    use crate::{GitService, LibGitService};
    use branchcraft_app_core::{BranchKind, GitErrorKind};
    use branchcraft_test_support::TestRepository;

    #[test]
    fn lists_local_and_remote_tracking_branches() {
        let repository = TestRepository::with_initial_commit();
        repository.add_remote_tracking_branch("origin/main");
        let service = LibGitService::new();
        service
            .create_branch(repository.path(), "feature/search")
            .expect("create branch");

        let branches = service.branches(repository.path()).expect("list branches");

        assert!(
            branches.iter().any(|branch| {
                branch.name == "feature/search" && branch.kind == BranchKind::Local
            })
        );
        assert!(
            branches.iter().any(|branch| {
                branch.name == "origin/main" && branch.kind == BranchKind::Remote
            })
        );
        assert_eq!(branches.iter().filter(|branch| branch.current).count(), 1);
    }

    #[test]
    fn creates_and_switches_local_branches() {
        let repository = TestRepository::with_initial_commit();
        let service = LibGitService::new();
        let original = service
            .open_repository(repository.path())
            .expect("open repository")
            .branch
            .display_name;

        let created = service
            .create_branch(repository.path(), "feature/work")
            .expect("create branch");
        assert_eq!(created.repository.branch.display_name, original);

        let switched = service
            .checkout_branch(repository.path(), "feature/work")
            .expect("switch branch");
        assert_eq!(switched.repository.branch.display_name, "feature/work");
        assert!(
            switched
                .branches
                .iter()
                .any(|branch| branch.name == "feature/work" && branch.current)
        );
    }

    #[test]
    fn safely_deletes_only_merged_non_current_branches() {
        let repository = TestRepository::with_initial_commit();
        let service = LibGitService::new();
        let original = service
            .open_repository(repository.path())
            .expect("open repository")
            .branch
            .display_name;
        service
            .create_branch(repository.path(), "merged")
            .expect("create merged branch");
        service
            .delete_branch(repository.path(), "merged")
            .expect("delete merged branch");

        let current_error = service
            .delete_branch(repository.path(), &original)
            .expect_err("current branch deletion must fail");
        assert_eq!(current_error.kind, GitErrorKind::UnsafeOperation);

        service
            .create_branch(repository.path(), "unmerged")
            .expect("create unmerged branch");
        service
            .checkout_branch(repository.path(), "unmerged")
            .expect("switch to unmerged branch");
        repository.write("feature.txt", "work\n");
        repository.stage("feature.txt");
        repository.commit("Unmerged work");
        service
            .checkout_branch(repository.path(), &original)
            .expect("switch to original branch");

        let unmerged_error = service
            .delete_branch(repository.path(), "unmerged")
            .expect_err("unmerged branch deletion must fail");
        assert_eq!(unmerged_error.kind, GitErrorKind::UnsafeOperation);
    }

    #[test]
    fn checkout_refuses_to_overwrite_working_tree_changes() {
        let repository = TestRepository::with_initial_commit();
        let service = LibGitService::new();
        let original = service
            .open_repository(repository.path())
            .expect("open repository")
            .branch
            .display_name;
        service
            .create_branch(repository.path(), "feature")
            .expect("create feature branch");
        service
            .checkout_branch(repository.path(), "feature")
            .expect("switch to feature");
        repository.write("README.md", "feature version\n");
        repository.stage("README.md");
        repository.commit("Feature change");
        service
            .checkout_branch(repository.path(), &original)
            .expect("switch to original");
        repository.write("README.md", "uncommitted version\n");

        let error = service
            .checkout_branch(repository.path(), "feature")
            .expect_err("conflicting worktree change must block checkout");

        assert_eq!(error.kind, GitErrorKind::DirtyWorkingTree);
        assert_eq!(repository.read("README.md"), "uncommitted version\n");
    }

    #[test]
    fn rejects_invalid_and_duplicate_branch_names() {
        let repository = TestRepository::with_initial_commit();
        let service = LibGitService::new();

        let invalid = service
            .create_branch(repository.path(), "bad branch")
            .expect_err("invalid branch must fail");
        assert_eq!(invalid.kind, GitErrorKind::InvalidReference);

        service
            .create_branch(repository.path(), "duplicate")
            .expect("create branch");
        let duplicate = service
            .create_branch(repository.path(), "duplicate")
            .expect_err("duplicate branch must fail");
        assert_eq!(duplicate.kind, GitErrorKind::BranchAlreadyExists);
    }
}
