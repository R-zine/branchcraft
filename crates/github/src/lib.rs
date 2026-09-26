//! GitHub-specific HTTP API operations for Branchcraft.

use async_trait::async_trait;
use branchcraft_app_core::{Account, GitHubRepository, GitHubRepositoryList};
use reqwest::{Client, StatusCode, Url, redirect::Policy};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

const API_VERSION: &str = "2026-03-10";
const DEFAULT_USER_ENDPOINT: &str = "https://api.github.com/user";
const DEFAULT_REPOSITORIES_ENDPOINT: &str = "https://api.github.com/user/repos";
const REPOSITORIES_PER_PAGE: usize = 100;
const MAX_REPOSITORY_PAGES: usize = 10;

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

    /// Lists repositories the authenticated user can access, newest activity
    /// first and bounded to avoid an unbounded desktop response.
    async fn repositories(
        &self,
        access_token: &SecretString,
    ) -> Result<GitHubRepositoryList, GitHubError>;
}

#[derive(Clone)]
pub struct ReqwestGitHubService {
    client: Client,
    user_endpoint: Url,
    repositories_endpoint: Url,
}

impl ReqwestGitHubService {
    /// Creates the production GitHub.com API client.
    ///
    /// # Errors
    ///
    /// Returns [`GitHubError::InvalidResponse`] if a fixed endpoint or the HTTP
    /// client cannot be initialized.
    pub fn new() -> Result<Self, GitHubError> {
        Self::with_endpoints(DEFAULT_USER_ENDPOINT, DEFAULT_REPOSITORIES_ENDPOINT)
    }

    /// Creates a client with an alternate endpoint for isolated tests.
    ///
    /// # Errors
    ///
    /// Returns [`GitHubError::InvalidResponse`] for an invalid endpoint.
    pub fn with_user_endpoint(endpoint: &str) -> Result<Self, GitHubError> {
        let user_endpoint = Url::parse(endpoint).map_err(|_| GitHubError::InvalidResponse)?;
        let mut repositories_endpoint = user_endpoint.clone();
        repositories_endpoint.set_path("/user/repos");
        repositories_endpoint.set_query(None);
        repositories_endpoint.set_fragment(None);
        Self::from_endpoints(user_endpoint, repositories_endpoint)
    }

    /// Creates a client with alternate user and repository endpoints for
    /// isolated tests.
    ///
    /// # Errors
    ///
    /// Returns [`GitHubError::InvalidResponse`] for invalid endpoints.
    pub fn with_endpoints(
        user_endpoint: &str,
        repositories_endpoint: &str,
    ) -> Result<Self, GitHubError> {
        let user_endpoint = Url::parse(user_endpoint).map_err(|_| GitHubError::InvalidResponse)?;
        let repositories_endpoint =
            Url::parse(repositories_endpoint).map_err(|_| GitHubError::InvalidResponse)?;
        Self::from_endpoints(user_endpoint, repositories_endpoint)
    }

    fn from_endpoints(user_endpoint: Url, repositories_endpoint: Url) -> Result<Self, GitHubError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("Branchcraft/0.1")
            .build()
            .map_err(|_| GitHubError::InvalidResponse)?;
        Ok(Self {
            client,
            user_endpoint,
            repositories_endpoint,
        })
    }

    async fn authenticated_get(
        &self,
        endpoint: Url,
        access_token: &SecretString,
    ) -> Result<reqwest::Response, GitHubError> {
        let response = self
            .client
            .get(endpoint)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .bearer_auth(access_token.expose_secret())
            .send()
            .await
            .map_err(|_| GitHubError::Network)?;

        match response.status() {
            StatusCode::UNAUTHORIZED => Err(GitHubError::Unauthorized),
            StatusCode::FORBIDDEN => Err(GitHubError::Forbidden),
            status if !status.is_success() => Err(GitHubError::Api),
            _ => Ok(response),
        }
    }
}

#[async_trait]
impl GitHubService for ReqwestGitHubService {
    async fn current_user(&self, access_token: &SecretString) -> Result<Account, GitHubError> {
        self.authenticated_get(self.user_endpoint.clone(), access_token)
            .await?
            .json::<GitHubUser>()
            .await
            .map(Account::from)
            .map_err(|_| GitHubError::InvalidResponse)
    }

    async fn repositories(
        &self,
        access_token: &SecretString,
    ) -> Result<GitHubRepositoryList, GitHubError> {
        let mut repositories = Vec::new();
        for page in 1..=MAX_REPOSITORY_PAGES {
            let mut endpoint = self.repositories_endpoint.clone();
            endpoint
                .query_pairs_mut()
                .append_pair("per_page", &REPOSITORIES_PER_PAGE.to_string())
                .append_pair("page", &page.to_string())
                .append_pair("sort", "pushed")
                .append_pair("direction", "desc");
            let response = self.authenticated_get(endpoint, access_token).await?;
            let page_repositories = response
                .json::<Vec<GitHubRepositoryResponse>>()
                .await
                .map_err(|_| GitHubError::InvalidResponse)?;
            let page_len = page_repositories.len();
            repositories.extend(
                page_repositories
                    .into_iter()
                    .map(GitHubRepository::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            );
            if page_len < REPOSITORIES_PER_PAGE {
                return Ok(GitHubRepositoryList {
                    repositories,
                    truncated: false,
                });
            }
        }

        Ok(GitHubRepositoryList {
            repositories,
            truncated: true,
        })
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

#[derive(Debug, Deserialize)]
struct GitHubRepositoryResponse {
    id: u64,
    name: String,
    full_name: String,
    owner: GitHubRepositoryOwner,
    description: Option<String>,
    private: bool,
    fork: bool,
    archived: bool,
    clone_url: String,
    default_branch: String,
    language: Option<String>,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRepositoryOwner {
    login: String,
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

impl TryFrom<GitHubRepositoryResponse> for GitHubRepository {
    type Error = GitHubError;

    fn try_from(repository: GitHubRepositoryResponse) -> Result<Self, Self::Error> {
        let clone_url =
            Url::parse(&repository.clone_url).map_err(|_| GitHubError::InvalidResponse)?;
        let valid_clone_url = clone_url.scheme() == "https"
            && clone_url
                .host_str()
                .is_some_and(|host| host.eq_ignore_ascii_case("github.com"))
            && clone_url.port_or_known_default() == Some(443)
            && clone_url.username().is_empty()
            && clone_url.password().is_none()
            && clone_url.query().is_none()
            && clone_url.fragment().is_none();
        if !valid_clone_url {
            return Err(GitHubError::InvalidResponse);
        }

        Ok(Self {
            id: repository.id,
            name: repository.name,
            full_name: repository.full_name,
            owner_login: repository.owner.login,
            description: repository.description,
            private: repository.private,
            fork: repository.fork,
            archived: repository.archived,
            clone_url: clone_url.to_string(),
            default_branch: repository.default_branch,
            language: repository.language,
            updated_at: repository.updated_at,
        })
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

    #[tokio::test]
    async fn lists_accessible_repositories_from_an_offline_mock() {
        let body = r#"[
          {
            "id": 7,
            "name": "branchcraft",
            "full_name": "branchcrafter/branchcraft",
            "owner": { "login": "branchcrafter" },
            "description": "A focused Git client",
            "private": true,
            "fork": false,
            "archived": false,
            "clone_url": "https://github.com/branchcrafter/branchcraft.git",
            "default_branch": "main",
            "language": "Rust",
            "updated_at": "2026-09-25T10:00:00Z"
          }
        ]"#;
        let (endpoint, request) = mock_endpoint("200 OK", body);
        let service =
            ReqwestGitHubService::with_endpoints(&endpoint, &endpoint).expect("GitHub client");

        let repositories = service
            .repositories(&SecretString::from("test-access-token"))
            .await
            .expect("repository listing");
        let request = request.join().expect("mock server").to_ascii_lowercase();

        assert!(!repositories.truncated);
        assert_eq!(repositories.repositories.len(), 1);
        assert_eq!(
            repositories.repositories[0].full_name,
            "branchcrafter/branchcraft"
        );
        assert!(repositories.repositories[0].private);
        assert!(request.starts_with("get /user?"));
        assert!(request.contains("per_page=100"));
        assert!(request.contains("sort=pushed"));
        assert!(request.contains("authorization: bearer test-access-token\r\n"));
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
