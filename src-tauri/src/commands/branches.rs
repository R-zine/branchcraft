use super::task_error;
use crate::AppState;
use branchcraft_app_core::{BranchOperationResult, BranchSummary, GitError};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchesRequest {
    repository_path: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchMutationRequest {
    repository_path: PathBuf,
    name: String,
}

#[tauri::command]
pub async fn list_branches(
    request: BranchesRequest,
    state: State<'_, AppState>,
) -> Result<Vec<BranchSummary>, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || git.branches(&request.repository_path))
        .await
        .map_err(|_| task_error())?
}

#[tauri::command]
pub async fn create_branch(
    request: BranchMutationRequest,
    state: State<'_, AppState>,
) -> Result<BranchOperationResult, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || {
        git.create_branch(&request.repository_path, &request.name)
    })
    .await
    .map_err(|_| task_error())?
}

#[tauri::command]
pub async fn checkout_branch(
    request: BranchMutationRequest,
    state: State<'_, AppState>,
) -> Result<BranchOperationResult, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || {
        git.checkout_branch(&request.repository_path, &request.name)
    })
    .await
    .map_err(|_| task_error())?
}

#[tauri::command]
pub async fn delete_branch(
    request: BranchMutationRequest,
    state: State<'_, AppState>,
) -> Result<BranchOperationResult, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || {
        git.delete_branch(&request.repository_path, &request.name)
    })
    .await
    .map_err(|_| task_error())?
}
