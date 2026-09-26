mod commands;

use branchcraft_app_core::{AuthError, AuthErrorKind};
use branchcraft_auth::{AuthService, BranchcraftAuth, BrowserLauncher, OAuthConfig};
use branchcraft_git::{GitService, LibGitService};
use branchcraft_github::{GitHubService, ReqwestGitHubService};
use branchcraft_storage::{KeyringSecretStore, SecretStore};
use std::sync::Arc;
use tauri::Manager;
use url::Url;

pub struct AppState {
    git: Arc<dyn GitService>,
    auth: Arc<dyn AuthService>,
}

impl AppState {
    fn new() -> Result<Self, AuthError> {
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
        let auth =
            BranchcraftAuth::github(OAuthConfig::from_environment(), github, secrets, browser)?;
        Ok(Self {
            git: Arc::new(LibGitService::new()),
            auth: Arc::new(auth),
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
            app.manage(AppState::new()?);
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
            commands::commits::create_commit,
            commands::diffs::get_file_diff,
            commands::history::get_commit_history,
            commands::repositories::open_repository,
            commands::repositories::refresh_repository,
            commands::staging::stage_file,
            commands::staging::unstage_file,
            commands::staging::stage_all,
            commands::staging::unstage_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running Branchcraft");
}
