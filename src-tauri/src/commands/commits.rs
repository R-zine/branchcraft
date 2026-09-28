use super::task_error;
use crate::AppState;
use branchcraft_app_core::{CommitResult, GitError};
use branchcraft_hooks::HookEvent;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::{State, ipc::Channel};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitRequest {
    repository_path: PathBuf,
    summary: String,
    description: Option<String>,
}

#[tauri::command]
pub async fn create_commit(
    request: CommitRequest,
    on_hook_event: Channel<HookEvent>,
    state: State<'_, AppState>,
) -> Result<CommitResult, GitError> {
    let git = Arc::clone(&state.git);
    tauri::async_runtime::spawn_blocking(move || {
        let mut report = |event| {
            let _ = on_hook_event.send(event);
        };
        git.commit(
            &request.repository_path,
            &request.summary,
            request.description.as_deref(),
            &mut report,
        )
    })
    .await
    .map_err(|_| task_error())?
}
