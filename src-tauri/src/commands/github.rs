use super::map_auth_error;
use crate::AppState;
use branchcraft_app_core::{GitError, GitErrorKind, GitHubRepositoryList};
use branchcraft_github::GitHubError;
use tauri::State;

#[tauri::command]
pub async fn list_github_repositories(
    state: State<'_, AppState>,
) -> Result<GitHubRepositoryList, GitError> {
    let token = state
        .auth
        .access_token()
        .await
        .map_err(|error| map_auth_error(&error))?;
    state
        .github
        .repositories(&token)
        .await
        .map_err(map_github_error)
}

fn map_github_error(error: GitHubError) -> GitError {
    match error {
        GitHubError::Unauthorized => GitError::new(
            GitErrorKind::Authentication,
            "GitHub rejected the stored credentials.",
            Some("Sign in to GitHub again, then reload repositories.".to_owned()),
        ),
        GitHubError::Forbidden => GitError::new(
            GitErrorKind::Authentication,
            "GitHub denied access to repository discovery.",
            Some("Review the OAuth authorization, then try again.".to_owned()),
        ),
        GitHubError::Network => GitError::new(
            GitErrorKind::Network,
            "GitHub repositories could not be loaded.",
            Some("Check your network connection, then try again.".to_owned()),
        ),
        GitHubError::InvalidResponse | GitHubError::Api => GitError::new(
            GitErrorKind::Unknown,
            "GitHub returned an unexpected repository response.",
            Some("Try again in a moment.".to_owned()),
        ),
    }
}
