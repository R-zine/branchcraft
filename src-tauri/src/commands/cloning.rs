use super::task_error;
use crate::AppState;
use branchcraft_app_core::{CloneProgress, GitError, RepositorySnapshot};
use branchcraft_git::CloneSourceKind;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::{State, ipc::Channel};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneRepositoryRequest {
    source_url: String,
    destination_parent: PathBuf,
}

#[tauri::command]
pub async fn clone_repository(
    request: CloneRepositoryRequest,
    on_progress: Channel<CloneProgress>,
    state: State<'_, AppState>,
) -> Result<RepositorySnapshot, GitError> {
    let git = Arc::clone(&state.git);
    let source_for_validation = request.source_url.clone();
    let source_kind =
        tauri::async_runtime::spawn_blocking(move || git.clone_source_kind(&source_for_validation))
            .await
            .map_err(|_| task_error())??;
    let token = if source_kind == CloneSourceKind::GitHubHttps {
        state.auth.access_token().await.ok()
    } else {
        None
    };
    let git = Arc::clone(&state.git);
    let settings = Arc::clone(&state.settings);

    tauri::async_runtime::spawn_blocking(move || {
        let mut report = |progress| {
            let _ = on_progress.send(progress);
        };
        let repository = git.clone_repository(
            &request.source_url,
            &request.destination_parent,
            token.as_ref(),
            &mut report,
        )?;
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
