pub mod diffs;
pub mod repositories;

use branchcraft_app_core::{GitError, GitErrorKind};

fn task_error() -> GitError {
    GitError::new(
        GitErrorKind::Unknown,
        "Branchcraft could not finish reading this repository.",
        Some("The background Git task stopped unexpectedly.".to_owned()),
    )
}
