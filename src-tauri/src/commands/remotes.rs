use super::{map_auth_error, task_error};
use crate::AppState;
use branchcraft_app_core::{
    GitError, RemoteInfo, RemoteKind, RemoteOperationResult, RemoteProgress,
};
use branchcraft_auth::AuthService;
use secrecy::SecretString;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::{State, ipc::Channel};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRequest {
    repository_path: PathBuf,
}

#[tauri::command]
pub async fn get_primary_remote(
    request: RemoteRequest,
    state: State<'_, AppState>,
) -> Result<Option<RemoteInfo>, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || git.primary_remote(&request.repository_path))
        .await
        .map_err(|_| task_error())?
}

#[tauri::command]
pub async fn fetch_remote(
    request: RemoteRequest,
    on_progress: Channel<RemoteProgress>,
    state: State<'_, AppState>,
) -> Result<RemoteOperationResult, GitError> {
    run_remote_operation(request, on_progress, &state, RemoteCommand::Fetch).await
}

#[tauri::command]
pub async fn pull_remote(
    request: RemoteRequest,
    on_progress: Channel<RemoteProgress>,
    state: State<'_, AppState>,
) -> Result<RemoteOperationResult, GitError> {
    run_remote_operation(request, on_progress, &state, RemoteCommand::Pull).await
}

#[tauri::command]
pub async fn push_remote(
    request: RemoteRequest,
    on_progress: Channel<RemoteProgress>,
    state: State<'_, AppState>,
) -> Result<RemoteOperationResult, GitError> {
    run_remote_operation(request, on_progress, &state, RemoteCommand::Push).await
}

#[derive(Clone, Copy)]
enum RemoteCommand {
    Fetch,
    Pull,
    Push,
}

async fn run_remote_operation(
    request: RemoteRequest,
    on_progress: Channel<RemoteProgress>,
    state: &State<'_, AppState>,
    command: RemoteCommand,
) -> Result<RemoteOperationResult, GitError> {
    let git = Arc::clone(&state.git);
    let repository_path = request.repository_path;
    let inspect_path = repository_path.clone();
    let remote = tauri::async_runtime::spawn_blocking(move || git.primary_remote(&inspect_path))
        .await
        .map_err(|_| task_error())??;
    let token = token_for_remote(remote.as_ref(), Arc::clone(&state.auth)).await?;
    let git = Arc::clone(&state.git);

    tauri::async_runtime::spawn_blocking(move || {
        let mut report = |progress| {
            let _ = on_progress.send(progress);
        };
        match command {
            RemoteCommand::Fetch => git.fetch(&repository_path, token.as_ref(), &mut report),
            RemoteCommand::Pull => git.pull(&repository_path, token.as_ref(), &mut report),
            RemoteCommand::Push => git.push(&repository_path, token.as_ref(), &mut report),
        }
    })
    .await
    .map_err(|_| task_error())?
}

async fn token_for_remote(
    remote: Option<&RemoteInfo>,
    auth: Arc<dyn AuthService>,
) -> Result<Option<SecretString>, GitError> {
    if !matches!(
        remote.map(|remote| remote.kind),
        Some(RemoteKind::GitHubHttps)
    ) {
        return Ok(None);
    }

    auth.access_token()
        .await
        .map(Some)
        .map_err(|error| map_auth_error(&error))
}
