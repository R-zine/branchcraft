mod commands;

use branchcraft_git::{GitService, LibGitService};
use std::sync::Arc;

pub struct AppState {
    git: Arc<dyn GitService>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            git: Arc::new(LibGitService::new()),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the Branchcraft desktop runtime.
///
/// # Panics
///
/// Panics if Tauri cannot initialize the application or its event loop.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::diffs::get_file_diff,
            commands::repositories::open_repository,
            commands::repositories::refresh_repository
        ])
        .run(tauri::generate_context!())
        .expect("error while running Branchcraft");
}
