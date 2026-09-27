use super::settings_task_error;
use crate::AppState;
use branchcraft_app_core::{ApplicationSettings, ColorPalette, SettingsError, ThemePreference};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetThemeRequest {
    theme: ThemePreference,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPaletteRequest {
    palette: ColorPalette,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentRepositoryRequest {
    path: PathBuf,
}

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<ApplicationSettings, SettingsError> {
    let settings = Arc::clone(&state.settings);
    tauri::async_runtime::spawn_blocking(move || settings.load())
        .await
        .map_err(|_| settings_task_error())?
}

#[tauri::command]
pub async fn set_theme(
    request: SetThemeRequest,
    state: State<'_, AppState>,
) -> Result<ApplicationSettings, SettingsError> {
    let settings = Arc::clone(&state.settings);
    tauri::async_runtime::spawn_blocking(move || settings.set_theme(request.theme))
        .await
        .map_err(|_| settings_task_error())?
}

#[tauri::command]
pub async fn set_palette(
    request: SetPaletteRequest,
    state: State<'_, AppState>,
) -> Result<ApplicationSettings, SettingsError> {
    let settings = Arc::clone(&state.settings);
    tauri::async_runtime::spawn_blocking(move || settings.set_palette(request.palette))
        .await
        .map_err(|_| settings_task_error())?
}

#[tauri::command]
pub async fn remove_recent_repository(
    request: RecentRepositoryRequest,
    state: State<'_, AppState>,
) -> Result<ApplicationSettings, SettingsError> {
    let settings = Arc::clone(&state.settings);
    tauri::async_runtime::spawn_blocking(move || settings.remove_recent_repository(&request.path))
        .await
        .map_err(|_| settings_task_error())?
}

#[tauri::command]
pub async fn clear_recent_repositories(
    state: State<'_, AppState>,
) -> Result<ApplicationSettings, SettingsError> {
    let settings = Arc::clone(&state.settings);
    tauri::async_runtime::spawn_blocking(move || settings.clear_recent_repositories())
        .await
        .map_err(|_| settings_task_error())?
}

#[tauri::command]
pub async fn reset_settings(
    state: State<'_, AppState>,
) -> Result<ApplicationSettings, SettingsError> {
    let settings = Arc::clone(&state.settings);
    tauri::async_runtime::spawn_blocking(move || settings.reset())
        .await
        .map_err(|_| settings_task_error())?
}
