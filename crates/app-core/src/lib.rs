//! Stable domain contracts shared by Branchcraft's backend modules.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// A stable identifier for an opened repository.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepositoryId(pub String);

/// The currently checked-out branch or detached commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub display_name: String,
    pub detached: bool,
    pub unborn: bool,
}

/// A change kind on one side of the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    TypeChanged,
    Untracked,
}

/// Status for one path, preserving index and worktree state independently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStatus {
    pub path: String,
    pub previous_path: Option<String>,
    pub staged: Option<ChangeKind>,
    pub unstaged: Option<ChangeKind>,
    pub conflicted: bool,
}

/// Everything needed to render the Milestone 1 repository workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositorySnapshot {
    pub id: RepositoryId,
    pub name: String,
    pub path: PathBuf,
    pub branch: BranchInfo,
    pub files: Vec<FileStatus>,
}

/// Error categories that frontend recovery UI can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitErrorKind {
    NotRepository,
    PermissionDenied,
    RepositoryNotFound,
    InvalidPath,
    Unknown,
}

/// A safe, structured error intended to cross the Tauri boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{summary}")]
#[serde(rename_all = "camelCase")]
pub struct GitError {
    pub kind: GitErrorKind,
    pub summary: String,
    pub detail: Option<String>,
}

impl GitError {
    #[must_use]
    pub fn new(kind: GitErrorKind, summary: impl Into<String>, detail: Option<String>) -> Self {
        Self {
            kind,
            summary: summary.into(),
            detail,
        }
    }
}
