pub mod auth;
pub mod branches;
pub mod cloning;
pub mod commits;
pub mod diffs;
pub mod github;
pub mod history;
pub mod remotes;
pub mod repositories;
pub mod settings;
pub mod staging;

use branchcraft_app_core::{
    AuthError, AuthErrorKind, GitError, GitErrorKind, SettingsError, SettingsErrorKind,
};

fn task_error() -> GitError {
    GitError::new(
        GitErrorKind::Unknown,
        "Branchcraft could not finish the Git operation.",
        Some("The background Git task stopped unexpectedly.".to_owned()),
    )
}

fn settings_task_error() -> SettingsError {
    SettingsError::new(
        SettingsErrorKind::Unknown,
        "Branchcraft could not finish the settings operation.",
        Some("The background settings task stopped unexpectedly.".to_owned()),
    )
}

fn map_auth_error(error: &AuthError) -> GitError {
    let (kind, summary, detail) = match error.kind {
        AuthErrorKind::Network => (
            GitErrorKind::Network,
            "GitHub could not be reached.",
            "Check your network connection, then try again.",
        ),
        AuthErrorKind::NotConfigured => (
            GitErrorKind::Authentication,
            "GitHub sign-in is not configured.",
            "Configure the Branchcraft OAuth App before using this feature.",
        ),
        AuthErrorKind::AuthenticationExpired => (
            GitErrorKind::Authentication,
            "Your GitHub sign-in has expired.",
            "Sign in to GitHub again, then retry the operation.",
        ),
        AuthErrorKind::CredentialStore => (
            GitErrorKind::Authentication,
            "GitHub credentials could not be loaded.",
            "Unlock the system keyring, then retry the operation.",
        ),
        _ => (
            GitErrorKind::Authentication,
            "GitHub authentication is required.",
            "Sign in to GitHub, then retry the operation.",
        ),
    };
    GitError::new(kind, summary, Some(detail.to_owned()))
}
