use crate::AppState;
use branchcraft_app_core::{Account, AuthError, AuthStatus};
use tauri::State;

#[tauri::command]
pub async fn get_auth_status(state: State<'_, AppState>) -> Result<AuthStatus, AuthError> {
    state.auth.status().await
}

#[tauri::command]
pub async fn login_github(state: State<'_, AppState>) -> Result<Account, AuthError> {
    state.auth.login().await
}

#[tauri::command]
pub async fn cancel_github_login(state: State<'_, AppState>) -> Result<(), AuthError> {
    state.auth.cancel_login().await;
    Ok(())
}

#[tauri::command]
pub async fn logout_github(state: State<'_, AppState>) -> Result<(), AuthError> {
    state.auth.logout().await
}
