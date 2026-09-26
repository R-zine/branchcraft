use crate::AppState;
use branchcraft_app_core::{GitError, GitErrorKind, RepositorySnapshot};
use serde::Deserialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRepositoryRequest {
    path: PathBuf,
}

#[tauri::command]
pub async fn open_repository(
    request: OpenRepositoryRequest,
    state: State<'_, AppState>,
) -> Result<RepositorySnapshot, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || git.open_repository(&request.path))
        .await
        .map_err(|_| task_error())?
}

#[tauri::command]
pub async fn refresh_repository(
    request: OpenRepositoryRequest,
    state: State<'_, AppState>,
) -> Result<RepositorySnapshot, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || git.repository_status(&request.path))
        .await
        .map_err(|_| task_error())?
}

fn task_error() -> GitError {
    GitError::new(
        GitErrorKind::Unknown,
        "Branchcraft could not finish reading this repository.",
        Some("The background Git task stopped unexpectedly.".to_owned()),
    )
}

use std::sync::Arc;
