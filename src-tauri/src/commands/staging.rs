use super::task_error;
use crate::AppState;
use branchcraft_app_core::{GitError, RepositorySnapshot};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMutationRequest {
    repository_path: PathBuf,
    file_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryMutationRequest {
    repository_path: PathBuf,
}

#[tauri::command]
pub async fn stage_file(
    request: FileMutationRequest,
    state: State<'_, AppState>,
) -> Result<RepositorySnapshot, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || {
        git.stage_file(&request.repository_path, &request.file_path)
    })
    .await
    .map_err(|_| task_error())?
}

#[tauri::command]
pub async fn unstage_file(
    request: FileMutationRequest,
    state: State<'_, AppState>,
) -> Result<RepositorySnapshot, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || {
        git.unstage_file(&request.repository_path, &request.file_path)
    })
    .await
    .map_err(|_| task_error())?
}

#[tauri::command]
pub async fn stage_all(
    request: RepositoryMutationRequest,
    state: State<'_, AppState>,
) -> Result<RepositorySnapshot, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || git.stage_all(&request.repository_path))
        .await
        .map_err(|_| task_error())?
}

#[tauri::command]
pub async fn unstage_all(
    request: RepositoryMutationRequest,
    state: State<'_, AppState>,
) -> Result<RepositorySnapshot, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || git.unstage_all(&request.repository_path))
        .await
        .map_err(|_| task_error())?
}
