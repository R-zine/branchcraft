use super::task_error;
use crate::AppState;
use branchcraft_app_core::{GitError, RepositorySnapshot};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
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
    let settings = Arc::clone(&state.settings);
    tauri::async_runtime::spawn_blocking(move || {
        let repository = git.open_repository(&request.path)?;
        if settings
            .record_recent_repository(&repository.name, &repository.path)
            .is_err()
        {
            log::warn!(target: "branchcraft", "Recent repository settings could not be updated");
        }
        Ok(repository)
    })
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
