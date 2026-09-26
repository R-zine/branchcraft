use super::task_error;
use crate::AppState;
use branchcraft_app_core::{CommitHistory, GitError};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRequest {
    repository_path: PathBuf,
}

#[tauri::command]
pub async fn get_commit_history(
    request: HistoryRequest,
    state: State<'_, AppState>,
) -> Result<CommitHistory, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || git.history(&request.repository_path))
        .await
        .map_err(|_| task_error())?
}
