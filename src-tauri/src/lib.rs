mod commands;
mod logging;

use branchcraft_app_core::{AuthError, AuthErrorKind};
use branchcraft_auth::{AuthService, BranchcraftAuth, BrowserLauncher, OAuthConfig};
use branchcraft_git::{GitService, LibGitService};
use branchcraft_github::{GitHubService, ReqwestGitHubService};
use branchcraft_storage::{JsonSettingsStore, KeyringSecretStore, SecretStore, SettingsStore};
use std::{path::PathBuf, sync::Arc};
use tauri::Manager;
use url::Url;

pub struct AppState {
    git: Arc<dyn GitService>,
    auth: Arc<dyn AuthService>,
    github: Arc<dyn GitHubService>,
    settings: Arc<dyn SettingsStore>,
}

impl AppState {
    fn new(settings_path: PathBuf) -> Result<Self, AuthError> {
        let github: Arc<dyn GitHubService> =
            Arc::new(ReqwestGitHubService::new().map_err(|_| {
                AuthError::new(
                    AuthErrorKind::Unknown,
                    "Branchcraft could not initialize GitHub authentication.",
                    None,
                )
            })?);
        let secrets: Arc<dyn SecretStore> = Arc::new(KeyringSecretStore::new("Branchcraft"));
        let browser: Arc<dyn BrowserLauncher> = Arc::new(SystemBrowser);
        let auth = BranchcraftAuth::github(
            OAuthConfig::from_environment(),
            Arc::clone(&github),
            secrets,
            browser,
        )?;
        Ok(Self {
            git: Arc::new(LibGitService::new()),
            auth: Arc::new(auth),
            github,
            settings: Arc::new(JsonSettingsStore::new(settings_path)),
        })
    }
}

struct SystemBrowser;

impl BrowserLauncher for SystemBrowser {
    fn open(&self, url: &Url) -> Result<(), AuthError> {
        tauri_plugin_opener::open_url(url.as_str(), None::<&str>).map_err(|_| {
            AuthError::new(
                AuthErrorKind::BrowserUnavailable,
                "Branchcraft could not open your web browser.",
                Some("Open a default browser, then try again.".to_owned()),
            )
        })
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
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_directory = app.path().app_config_dir()?;
            if let Ok(log_directory) = app.path().app_log_dir() {
                if logging::initialize(&log_directory).is_ok() {
                    log::info!(target: "branchcraft", "Branchcraft desktop runtime started");
                } else {
                    eprintln!("Branchcraft production logging is unavailable.");
                }
            } else {
                eprintln!("Branchcraft production logging is unavailable.");
            }
            std::panic::set_hook(Box::new(|_| {
                log::error!(target: "branchcraft", "Branchcraft encountered an unexpected panic");
                eprintln!("Branchcraft encountered an unexpected error.");
            }));
            app.manage(AppState::new(config_directory.join("settings.json"))?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::get_auth_status,
            commands::auth::login_github,
            commands::auth::cancel_github_login,
            commands::auth::logout_github,
            commands::branches::list_branches,
            commands::branches::create_branch,
            commands::branches::checkout_branch,
            commands::branches::delete_branch,
            commands::cloning::clone_repository,
            commands::commits::create_commit,
            commands::diffs::get_file_diff,
            commands::github::list_github_repositories,
            commands::history::get_commit_history,
            commands::remotes::get_primary_remote,
            commands::remotes::fetch_remote,
            commands::remotes::pull_remote,
            commands::remotes::push_remote,
            commands::repositories::open_repository,
            commands::repositories::refresh_repository,
            commands::settings::get_settings,
            commands::settings::set_theme,
            commands::settings::set_palette,
            commands::settings::remove_recent_repository,
            commands::settings::clear_recent_repositories,
            commands::settings::reset_settings,
            commands::staging::stage_file,
            commands::staging::unstage_file,
            commands::staging::stage_all,
            commands::staging::unstage_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running Branchcraft");
}
