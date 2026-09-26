use super::task_error;
use crate::AppState;
use branchcraft_app_core::{Diff, GitError};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiffRequest {
    repository_path: PathBuf,
    file_path: String,
}

#[tauri::command]
pub async fn get_file_diff(
    request: FileDiffRequest,
    state: State<'_, AppState>,
) -> Result<Diff, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || {
        git.diff(&request.repository_path, &request.file_path)
    })
    .await
    .map_err(|_| task_error())?
}
