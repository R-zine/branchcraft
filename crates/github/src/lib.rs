//! GitHub-specific HTTP API operations for Branchcraft.

use async_trait::async_trait;
use branchcraft_app_core::Account;
use reqwest::{Client, StatusCode, Url};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

const API_VERSION: &str = "2026-03-10";
const DEFAULT_USER_ENDPOINT: &str = "https://api.github.com/user";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum GitHubError {
    #[error("GitHub rejected the stored credentials")]
    Unauthorized,
    #[error("GitHub denied access to this resource")]
    Forbidden,
    #[error("GitHub could not be reached")]
    Network,
    #[error("GitHub returned an invalid response")]
    InvalidResponse,
    #[error("GitHub returned an unsuccessful response")]
    Api,
}

#[async_trait]
pub trait GitHubService: Send + Sync {
    /// Retrieves and validates the identity represented by `access_token`.
    async fn current_user(&self, access_token: &SecretString) -> Result<Account, GitHubError>;
}

#[derive(Clone)]
pub struct ReqwestGitHubService {
    client: Client,
    user_endpoint: Url,
}

impl ReqwestGitHubService {
    /// Creates the production GitHub.com API client.
    ///
    /// # Errors
    ///
    /// Returns [`GitHubError::InvalidResponse`] if a fixed endpoint or the HTTP
    /// client cannot be initialized.
    pub fn new() -> Result<Self, GitHubError> {
        Self::with_user_endpoint(DEFAULT_USER_ENDPOINT)
    }

    /// Creates a client with an alternate endpoint for isolated tests.
    ///
    /// # Errors
    ///
    /// Returns [`GitHubError::InvalidResponse`] for an invalid endpoint.
    pub fn with_user_endpoint(endpoint: &str) -> Result<Self, GitHubError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("Branchcraft/0.1")
            .build()
            .map_err(|_| GitHubError::InvalidResponse)?;
        let user_endpoint = Url::parse(endpoint).map_err(|_| GitHubError::InvalidResponse)?;
        Ok(Self {
            client,
            user_endpoint,
        })
    }
}

#[async_trait]
impl GitHubService for ReqwestGitHubService {
    async fn current_user(&self, access_token: &SecretString) -> Result<Account, GitHubError> {
        let response = self
            .client
            .get(self.user_endpoint.clone())
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .bearer_auth(access_token.expose_secret())
            .send()
            .await
            .map_err(|_| GitHubError::Network)?;

        match response.status() {
            StatusCode::UNAUTHORIZED => return Err(GitHubError::Unauthorized),
            StatusCode::FORBIDDEN => return Err(GitHubError::Forbidden),
            status if !status.is_success() => return Err(GitHubError::Api),
            _ => {}
        }

        response
            .json::<GitHubUser>()
            .await
            .map(Account::from)
            .map_err(|_| GitHubError::InvalidResponse)
    }
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    id: u64,
    login: String,
    name: Option<String>,
    avatar_url: String,
    html_url: String,
}

impl From<GitHubUser> for Account {
    fn from(user: GitHubUser) -> Self {
        Self {
            id: user.id,
            login: user.login,
            name: user.name,
            avatar_url: user.avatar_url,
            profile_url: user.html_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{API_VERSION, GitHubError, GitHubService, GitHubUser, ReqwestGitHubService};
    use branchcraft_app_core::Account;
    use secrecy::SecretString;
    use std::{
        io::{Read, Write},
        net::{Ipv4Addr, TcpListener},
        thread::{self, JoinHandle},
        time::Duration,
    };

    #[test]
    fn converts_only_public_profile_fields() {
        let account = Account::from(GitHubUser {
            id: 42,
            login: "branchcrafter".to_owned(),
            name: Some("Branch Crafter".to_owned()),
            avatar_url: "https://avatars.githubusercontent.com/u/42".to_owned(),
            html_url: "https://github.com/branchcrafter".to_owned(),
        });

        assert_eq!(account.id, 42);
        assert_eq!(account.login, "branchcrafter");
        assert_eq!(account.name.as_deref(), Some("Branch Crafter"));
    }

    #[tokio::test]
    async fn requests_the_authenticated_user_from_an_offline_mock() {
        let body = r#"{"id":42,"login":"branchcrafter","name":"Branch Crafter","avatar_url":"https://avatars.example/42","html_url":"https://github.com/branchcrafter"}"#;
        let (endpoint, request) = mock_endpoint("200 OK", body);
        let service = ReqwestGitHubService::with_user_endpoint(&endpoint).expect("GitHub client");

        let account = service
            .current_user(&SecretString::from("test-access-token"))
            .await
            .expect("authenticated user");
        let request = request.join().expect("mock server").to_ascii_lowercase();

        assert_eq!(account.login, "branchcrafter");
        assert!(request.starts_with("get /user http/1.1\r\n"));
        assert!(request.contains("authorization: bearer test-access-token\r\n"));
        assert!(request.contains(&format!(
            "x-github-api-version: {}\r\n",
            API_VERSION.to_ascii_lowercase()
        )));
    }

    #[tokio::test]
    async fn maps_rejected_credentials_without_exposing_the_response() {
        let (endpoint, request) = mock_endpoint("401 Unauthorized", "");
        let service = ReqwestGitHubService::with_user_endpoint(&endpoint).expect("GitHub client");

        let error = service
            .current_user(&SecretString::from("rejected-test-token"))
            .await
            .expect_err("credentials should be rejected");
        request.join().expect("mock server");

        assert_eq!(error, GitHubError::Unauthorized);
    }

    fn mock_endpoint(status: &str, body: &str) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("mock listener");
        let address = listener.local_addr().expect("mock address");
        let status = status.to_owned();
        let body = body.to_owned();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("mock connection");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("mock timeout");
            let mut request = Vec::new();
            loop {
                let mut chunk = [0_u8; 1024];
                let read = stream.read(&mut chunk).expect("mock request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("mock response");
            String::from_utf8(request).expect("HTTP request text")
        });
        (format!("http://{address}/user"), server)
    }
}
