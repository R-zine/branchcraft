pub mod auth;
pub mod branches;
pub mod commits;
pub mod diffs;
pub mod history;
pub mod remotes;
pub mod repositories;
pub mod staging;

use branchcraft_app_core::{GitError, GitErrorKind};

fn task_error() -> GitError {
    GitError::new(
        GitErrorKind::Unknown,
        "Branchcraft could not finish the Git operation.",
        Some("The background Git task stopped unexpectedly.".to_owned()),
    )
}
