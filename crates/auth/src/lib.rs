//! GitHub OAuth authorization and session management for Branchcraft.

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use branchcraft_app_core::{Account, AuthError, AuthErrorKind, AuthStatus};
use branchcraft_github::{GitHubError, GitHubService};
use branchcraft_storage::{SecretStore, StorageError};
use reqwest::{Client, Url, redirect::Policy};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    net::Ipv4Addr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Mutex, oneshot},
    time::timeout,
};

pub const GITHUB_CREDENTIAL_KEY: &str = "github-oauth-session";
const CALLBACK_PATH: &str = "/oauth/callback";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
const CALLBACK_READ_TIMEOUT: Duration = Duration::from_secs(10);
const REFRESH_MARGIN_SECONDS: u64 = 60;
const GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// Public OAuth App configuration. GitHub requires native public clients to
/// ship the OAuth App client secret, so it is redacted in memory but is not
/// treated as a confidential security boundary. PKCE secures authorization
/// codes issued to this client.
#[derive(Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: SecretString,
}

impl OAuthConfig {
    #[must_use]
    pub fn new(client_id: impl Into<String>, client_secret: SecretString) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret,
        }
    }

    /// Loads runtime configuration first, then build-time configuration.
    /// Empty values are treated as unconfigured.
    #[must_use]
    pub fn from_environment() -> Option<Self> {
        let client_id = environment_or_build(
            "BRANCHCRAFT_GITHUB_CLIENT_ID",
            option_env!("BRANCHCRAFT_GITHUB_CLIENT_ID"),
        )?;
        let client_secret = environment_or_build(
            "BRANCHCRAFT_GITHUB_CLIENT_SECRET",
            option_env!("BRANCHCRAFT_GITHUB_CLIENT_SECRET"),
        )?;
        Some(Self::new(client_id, SecretString::from(client_secret)))
    }
}

fn environment_or_build(name: &str, build_value: Option<&str>) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| build_value.map(ToOwned::to_owned))
        .filter(|value| !value.trim().is_empty())
}

#[async_trait]
pub trait AuthService: Send + Sync {
    /// Returns configuration and account state without exposing credentials.
    ///
    /// # Errors
    ///
    /// Returns a structured error if the keyring or GitHub cannot be reached.
    async fn status(&self) -> Result<AuthStatus, AuthError>;

    /// Runs browser-based authorization and returns the authenticated account.
    ///
    /// # Errors
    ///
    /// Returns a structured error for configuration, listener, browser,
    /// authorization, network, API, or credential-store failures.
    async fn login(&self) -> Result<Account, AuthError>;

    /// Cancels a pending browser authorization, if any.
    async fn cancel_login(&self);

    /// Removes locally stored credentials. This does not revoke the OAuth App
    /// authorization on GitHub.
    ///
    /// # Errors
    ///
    /// Returns a structured error if the credential store cannot be updated.
    async fn logout(&self) -> Result<(), AuthError>;

    /// Resolves the account represented by stored credentials.
    ///
    /// # Errors
    ///
    /// Returns a structured error if refresh, storage, or GitHub validation
    /// fails.
    async fn current_account(&self) -> Result<Option<Account>, AuthError>;

    /// Returns a backend-only access token, refreshing it when necessary.
    ///
    /// # Errors
    ///
    /// Returns a structured error when there is no session or it cannot be
    /// refreshed safely.
    async fn access_token(&self) -> Result<SecretString, AuthError>;
}

pub trait BrowserLauncher: Send + Sync {
    /// Opens an HTTPS authorization URL in the user's default browser.
    ///
    /// # Errors
    ///
    /// Returns a safe error if no browser can be launched.
    fn open(&self, url: &Url) -> Result<(), AuthError>;
}

/// Tokens returned by the OAuth provider. Secret fields have redacted debug
/// behavior and are never serialized across IPC.
#[derive(Clone)]
pub struct OAuthTokenSet {
    pub access_token: SecretString,
    pub refresh_token: Option<SecretString>,
    pub expires_in: Option<u64>,
    pub refresh_token_expires_in: Option<u64>,
}

#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Builds the external authorization URL.
    ///
    /// # Errors
    ///
    /// Returns a safe error if the configured endpoint is invalid.
    fn authorization_url(
        &self,
        redirect_uri: &Url,
        state: &str,
        code_challenge: &str,
    ) -> Result<Url, AuthError>;

    /// Exchanges a one-time authorization code using the PKCE verifier.
    ///
    /// # Errors
    ///
    /// Returns a safe transport or token-response error.
    async fn exchange_code(
        &self,
        code: &SecretString,
        redirect_uri: &Url,
        code_verifier: &SecretString,
    ) -> Result<OAuthTokenSet, AuthError>;

    /// Rotates an expiring access/refresh token pair.
    ///
    /// # Errors
    ///
    /// Returns a safe transport, expiration, or token-response error.
    async fn refresh(&self, refresh_token: &SecretString) -> Result<OAuthTokenSet, AuthError>;
}

#[derive(Clone)]
pub struct GitHubOAuthProvider {
    config: OAuthConfig,
    client: Client,
    authorize_url: Url,
    token_url: Url,
}

impl GitHubOAuthProvider {
    /// Creates the production GitHub.com OAuth client.
    ///
    /// # Errors
    ///
    /// Returns a safe configuration error if fixed endpoints or the HTTP
    /// client cannot be initialized.
    pub fn new(config: OAuthConfig) -> Result<Self, AuthError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("Branchcraft/0.1")
            .build()
            .map_err(|_| invalid_oauth_configuration())?;
        Ok(Self {
            config,
            client,
            authorize_url: Url::parse(GITHUB_AUTHORIZE_URL)
                .map_err(|_| invalid_oauth_configuration())?,
            token_url: Url::parse(GITHUB_TOKEN_URL).map_err(|_| invalid_oauth_configuration())?,
        })
    }
}

#[async_trait]
impl OAuthProvider for GitHubOAuthProvider {
    fn authorization_url(
        &self,
        redirect_uri: &Url,
        state: &str,
        code_challenge: &str,
    ) -> Result<Url, AuthError> {
        let mut url = self.authorize_url.clone();
        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", redirect_uri.as_str())
            .append_pair("scope", "repo offline_access")
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(url)
    }

    async fn exchange_code(
        &self,
        code: &SecretString,
        redirect_uri: &Url,
        code_verifier: &SecretString,
    ) -> Result<OAuthTokenSet, AuthError> {
        self.request_tokens(&[
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.expose_secret()),
            ("code", code.expose_secret()),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", code_verifier.expose_secret()),
        ])
        .await
    }

    async fn refresh(&self, refresh_token: &SecretString) -> Result<OAuthTokenSet, AuthError> {
        self.request_tokens(&[
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.expose_secret()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.expose_secret()),
        ])
        .await
    }
}

impl GitHubOAuthProvider {
    async fn request_tokens(&self, form: &[(&str, &str)]) -> Result<OAuthTokenSet, AuthError> {
        let response = self
            .client
            .post(self.token_url.clone())
            .header("Accept", "application/json")
            .form(form)
            .send()
            .await
            .map_err(|_| network_error("GitHub's token endpoint could not be reached."))?;
        let status = response.status();
        let payload = response
            .json::<OAuthTokenResponse>()
            .await
            .map_err(|_| token_response_error())?;

        if let Some(error) = payload.error.as_deref() {
            return Err(match error {
                "bad_refresh_token" | "expired_token" => AuthError::new(
                    AuthErrorKind::AuthenticationExpired,
                    "Your GitHub session has expired.",
                    Some("Sign in again to continue using GitHub features.".to_owned()),
                ),
                "access_denied" => AuthError::new(
                    AuthErrorKind::AuthorizationDenied,
                    "GitHub authorization was denied.",
                    None,
                ),
                _ => token_response_error(),
            });
        }
        if !status.is_success() {
            return Err(token_response_error());
        }
        let access_token = payload
            .access_token
            .filter(|token| !token.is_empty())
            .ok_or_else(token_response_error)?;
        if !payload
            .token_type
            .as_deref()
            .is_some_and(|kind| kind.eq_ignore_ascii_case("bearer"))
        {
            return Err(token_response_error());
        }
        Ok(OAuthTokenSet {
            access_token: SecretString::from(access_token),
            refresh_token: payload.refresh_token.map(SecretString::from),
            expires_in: payload.expires_in,
            refresh_token_expires_in: payload.refresh_token_expires_in,
        })
    }
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    refresh_token_expires_in: Option<u64>,
    error: Option<String>,
}

pub struct BranchcraftAuth {
    configured: bool,
    oauth: Option<Arc<dyn OAuthProvider>>,
    github: Arc<dyn GitHubService>,
    secrets: Arc<dyn SecretStore>,
    browser: Arc<dyn BrowserLauncher>,
    login_cancel: Mutex<Option<oneshot::Sender<()>>>,
    token_refresh: Mutex<()>,
}

impl BranchcraftAuth {
    #[must_use]
    pub fn with_dependencies(
        configured: bool,
        oauth: Option<Arc<dyn OAuthProvider>>,
        github: Arc<dyn GitHubService>,
        secrets: Arc<dyn SecretStore>,
        browser: Arc<dyn BrowserLauncher>,
    ) -> Self {
        Self {
            configured,
            oauth,
            github,
            secrets,
            browser,
            login_cancel: Mutex::new(None),
            token_refresh: Mutex::new(()),
        }
    }

    /// Creates an auth service using GitHub's production OAuth endpoints.
    ///
    /// # Errors
    ///
    /// Returns a configuration error if the HTTP client cannot be initialized.
    pub fn github(
        config: Option<OAuthConfig>,
        github: Arc<dyn GitHubService>,
        secrets: Arc<dyn SecretStore>,
        browser: Arc<dyn BrowserLauncher>,
    ) -> Result<Self, AuthError> {
        let configured = config.is_some();
        let oauth = config
            .map(GitHubOAuthProvider::new)
            .transpose()?
            .map(|provider| Arc::new(provider) as Arc<dyn OAuthProvider>);
        Ok(Self::with_dependencies(
            configured, oauth, github, secrets, browser,
        ))
    }

    async fn login_flow(&self) -> Result<Account, AuthError> {
        let oauth = self.oauth.as_ref().ok_or_else(not_configured)?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|_| listener_error())?;
        let port = listener.local_addr().map_err(|_| listener_error())?.port();
        let redirect_uri = Url::parse(&format!(
            "http://{}:{port}{CALLBACK_PATH}",
            Ipv4Addr::LOCALHOST
        ))
        .map_err(|_| listener_error())?;
        let state = SecretString::from(random_token());
        let verifier = SecretString::from(random_token());
        let challenge = pkce_challenge(&verifier);
        let authorization_url =
            oauth.authorization_url(&redirect_uri, state.expose_secret(), &challenge)?;
        self.browser.open(&authorization_url)?;

        let mut callback = timeout(
            LOGIN_TIMEOUT,
            receive_callback(listener, state.expose_secret()),
        )
        .await
        .map_err(|_| {
            AuthError::new(
                AuthErrorKind::Cancelled,
                "GitHub sign-in timed out.",
                Some("Start sign-in again when you are ready.".to_owned()),
            )
        })??;

        let result = async {
            let tokens = oauth
                .exchange_code(&callback.code, &redirect_uri, &verifier)
                .await?;
            let account = self
                .github
                .current_user(&tokens.access_token)
                .await
                .map_err(map_github)?;
            self.store_tokens(tokens).await?;
            Ok(account)
        }
        .await;
        let _ = callback.respond(result.is_ok()).await;
        result
    }

    async fn load_tokens(&self) -> Result<Option<StoredTokens>, AuthError> {
        self.secrets
            .get(GITHUB_CREDENTIAL_KEY)
            .await
            .map_err(map_storage)?
            .map(|stored| {
                serde_json::from_str(stored.expose_secret()).map_err(|_| {
                    AuthError::new(
                        AuthErrorKind::CredentialStore,
                        "The stored GitHub session could not be read.",
                        Some("Sign out, then sign in again.".to_owned()),
                    )
                })
            })
            .transpose()
    }

    async fn store_tokens(&self, tokens: OAuthTokenSet) -> Result<(), AuthError> {
        let stored = StoredTokens::from_oauth(tokens);
        let encoded = serde_json::to_string(&stored).map_err(|_| {
            AuthError::new(
                AuthErrorKind::Unknown,
                "Branchcraft could not prepare the GitHub session.",
                None,
            )
        })?;
        self.secrets
            .set(GITHUB_CREDENTIAL_KEY, &SecretString::from(encoded))
            .await
            .map_err(map_storage)
    }

    async fn fresh_access_token(&self) -> Result<Option<SecretString>, AuthError> {
        let _refresh_guard = self.token_refresh.lock().await;
        let Some(tokens) = self.load_tokens().await? else {
            return Ok(None);
        };
        if !tokens.access_expired() {
            return Ok(Some(SecretString::from(tokens.access_token)));
        }

        let Some(refresh_token) = tokens.refresh_token.as_ref() else {
            self.secrets
                .delete(GITHUB_CREDENTIAL_KEY)
                .await
                .map_err(map_storage)?;
            return Err(expired_error());
        };
        if tokens.refresh_expired() {
            self.secrets
                .delete(GITHUB_CREDENTIAL_KEY)
                .await
                .map_err(map_storage)?;
            return Err(expired_error());
        }
        let oauth = self.oauth.as_ref().ok_or_else(not_configured)?;
        let refreshed = match oauth
            .refresh(&SecretString::from(refresh_token.clone()))
            .await
        {
            Ok(tokens) => tokens,
            Err(error) => {
                if error.kind == AuthErrorKind::AuthenticationExpired {
                    self.secrets
                        .delete(GITHUB_CREDENTIAL_KEY)
                        .await
                        .map_err(map_storage)?;
                }
                return Err(error);
            }
        };
        let access_token = refreshed.access_token.clone();
        self.store_tokens(refreshed).await?;
        Ok(Some(access_token))
    }
}

#[async_trait]
impl AuthService for BranchcraftAuth {
    async fn status(&self) -> Result<AuthStatus, AuthError> {
        if !self.configured {
            return Ok(AuthStatus::Unconfigured);
        }
        Ok(match self.current_account().await? {
            Some(account) => AuthStatus::SignedIn { account },
            None => AuthStatus::SignedOut,
        })
    }

    async fn login(&self) -> Result<Account, AuthError> {
        if !self.configured || self.oauth.is_none() {
            return Err(not_configured());
        }
        let (cancel_sender, mut cancel_receiver) = oneshot::channel();
        {
            let mut active = self.login_cancel.lock().await;
            if active.is_some() {
                return Err(AuthError::new(
                    AuthErrorKind::AlreadyInProgress,
                    "GitHub sign-in is already in progress.",
                    None,
                ));
            }
            *active = Some(cancel_sender);
        }

        let result = tokio::select! {
            result = self.login_flow() => result,
            _ = &mut cancel_receiver => Err(AuthError::new(
                AuthErrorKind::Cancelled,
                "GitHub sign-in was cancelled.",
                None,
            )),
        };
        self.login_cancel.lock().await.take();
        result
    }

    async fn cancel_login(&self) {
        if let Some(cancel) = self.login_cancel.lock().await.take() {
            let _ = cancel.send(());
        }
    }

    async fn logout(&self) -> Result<(), AuthError> {
        self.cancel_login().await;
        let _refresh_guard = self.token_refresh.lock().await;
        self.secrets
            .delete(GITHUB_CREDENTIAL_KEY)
            .await
            .map_err(map_storage)
    }

    async fn current_account(&self) -> Result<Option<Account>, AuthError> {
        let Some(access_token) = self.fresh_access_token().await? else {
            return Ok(None);
        };
        match self.github.current_user(&access_token).await {
            Ok(account) => Ok(Some(account)),
            Err(GitHubError::Unauthorized) => {
                self.secrets
                    .delete(GITHUB_CREDENTIAL_KEY)
                    .await
                    .map_err(map_storage)?;
                Ok(None)
            }
            Err(error) => Err(map_github(error)),
        }
    }

    async fn access_token(&self) -> Result<SecretString, AuthError> {
        self.fresh_access_token().await?.ok_or_else(|| {
            AuthError::new(
                AuthErrorKind::NotAuthenticated,
                "Sign in with GitHub to continue.",
                None,
            )
        })
    }
}

#[derive(Serialize, Deserialize)]
struct StoredTokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
    refresh_expires_at: Option<u64>,
}

impl StoredTokens {
    fn from_oauth(tokens: OAuthTokenSet) -> Self {
        let now = unix_seconds();
        Self {
            access_token: tokens.access_token.expose_secret().to_owned(),
            refresh_token: tokens
                .refresh_token
                .map(|token| token.expose_secret().to_owned()),
            expires_at: tokens.expires_in.map(|seconds| now.saturating_add(seconds)),
            refresh_expires_at: tokens
                .refresh_token_expires_in
                .map(|seconds| now.saturating_add(seconds)),
        }
    }

    fn access_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expiry| expiry <= unix_seconds().saturating_add(REFRESH_MARGIN_SECONDS))
    }

    fn refresh_expired(&self) -> bool {
        self.refresh_expires_at
            .is_some_and(|expiry| expiry <= unix_seconds())
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn random_token() -> String {
    URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}

fn pkce_challenge(verifier: &SecretString) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.expose_secret().as_bytes()))
}

struct PendingCallback {
    code: SecretString,
    stream: TcpStream,
}

impl PendingCallback {
    async fn respond(&mut self, success: bool) -> std::io::Result<()> {
        let (title, message) = if success {
            (
                "Authentication complete",
                "Authentication complete. You can return to Branchcraft.",
            )
        } else {
            (
                "Authentication failed",
                "Branchcraft could not complete authentication. Return to the app for details.",
            )
        };
        write_browser_response(&mut self.stream, title, message).await
    }
}

async fn receive_callback(
    listener: TcpListener,
    expected_state: &str,
) -> Result<PendingCallback, AuthError> {
    let (mut stream, peer) = listener.accept().await.map_err(|_| listener_error())?;
    if !peer.ip().is_loopback() {
        return Err(listener_error());
    }
    let request = read_http_request(&mut stream).await?;
    match parse_callback(&request, expected_state) {
        Ok(code) => Ok(PendingCallback { code, stream }),
        Err(error) => {
            let _ = write_browser_response(
                &mut stream,
                "Authentication stopped",
                "Authentication did not complete. You can return to Branchcraft.",
            )
            .await;
            Err(error)
        }
    }
}

async fn read_http_request(stream: &mut TcpStream) -> Result<Vec<u8>, AuthError> {
    let mut request = Vec::with_capacity(1024);
    timeout(CALLBACK_READ_TIMEOUT, async {
        loop {
            let mut chunk = [0_u8; 1024];
            let read = stream
                .read(&mut chunk)
                .await
                .map_err(|_| listener_error())?;
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.len() > 8192 {
                return Err(invalid_callback());
            }
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        Ok::<(), AuthError>(())
    })
    .await
    .map_err(|_| invalid_callback())??;
    Ok(request)
}

fn parse_callback(request: &[u8], expected_state: &str) -> Result<SecretString, AuthError> {
    let request = std::str::from_utf8(request).map_err(|_| invalid_callback())?;
    let request_line = request.lines().next().ok_or_else(invalid_callback)?;
    let mut parts = request_line.split_whitespace();
    if parts.next() != Some("GET") {
        return Err(invalid_callback());
    }
    let target = parts.next().ok_or_else(invalid_callback)?;
    let callback =
        Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| invalid_callback())?;
    if callback.path() != CALLBACK_PATH {
        return Err(invalid_callback());
    }
    let mut code = None;
    let mut state = None;
    let mut oauth_error = None;
    for (name, value) in callback.query_pairs() {
        match name.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => oauth_error = Some(value.into_owned()),
            _ => {}
        }
    }
    let state = state.ok_or_else(state_mismatch)?;
    if expected_state
        .as_bytes()
        .ct_eq(state.as_bytes())
        .unwrap_u8()
        != 1
    {
        return Err(state_mismatch());
    }
    if let Some(error) = oauth_error {
        return Err(if error == "access_denied" {
            AuthError::new(
                AuthErrorKind::Cancelled,
                "GitHub sign-in was cancelled.",
                None,
            )
        } else {
            AuthError::new(
                AuthErrorKind::AuthorizationDenied,
                "GitHub could not authorize Branchcraft.",
                Some("Review the authorization request and try again.".to_owned()),
            )
        });
    }
    code.filter(|value| !value.is_empty())
        .map(SecretString::from)
        .ok_or_else(invalid_callback)
}

async fn write_browser_response(
    stream: &mut TcpStream,
    title: &str,
    message: &str,
) -> std::io::Result<()> {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\"><title>{title}</title><style>body{{margin:0;background:#0e1218;color:#edf1f7;font:16px system-ui;display:grid;min-height:100vh;place-items:center}}main{{max-width:34rem;padding:2rem;text-align:center}}h1{{font-size:1.4rem}}p{{color:#aab4c0;line-height:1.6}}</style></head><body><main><h1>{title}</h1><p>{message}</p></main></body></html>"
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}

fn not_configured() -> AuthError {
    AuthError::new(
        AuthErrorKind::NotConfigured,
        "GitHub sign-in is not configured in this build.",
        Some("Build Branchcraft with its GitHub OAuth App configuration.".to_owned()),
    )
}

fn invalid_oauth_configuration() -> AuthError {
    AuthError::new(
        AuthErrorKind::NotConfigured,
        "The GitHub OAuth configuration is invalid.",
        None,
    )
}

fn listener_error() -> AuthError {
    AuthError::new(
        AuthErrorKind::ListenerUnavailable,
        "Branchcraft could not start the sign-in callback.",
        Some("Check local firewall settings and try again.".to_owned()),
    )
}

fn invalid_callback() -> AuthError {
    AuthError::new(
        AuthErrorKind::InvalidResponse,
        "Branchcraft received an invalid sign-in callback.",
        Some("Close the browser page and start sign-in again.".to_owned()),
    )
}

fn state_mismatch() -> AuthError {
    AuthError::new(
        AuthErrorKind::StateMismatch,
        "Branchcraft rejected an unexpected sign-in callback.",
        Some("No credentials were stored. Start sign-in again.".to_owned()),
    )
}

fn token_response_error() -> AuthError {
    AuthError::new(
        AuthErrorKind::TokenExchange,
        "GitHub did not issue a valid access token.",
        Some(
            "Start sign-in again. If the problem continues, verify the OAuth App configuration."
                .to_owned(),
        ),
    )
}

fn expired_error() -> AuthError {
    AuthError::new(
        AuthErrorKind::AuthenticationExpired,
        "Your GitHub session has expired.",
        Some("Sign in again to continue using GitHub features.".to_owned()),
    )
}

fn network_error(detail: &str) -> AuthError {
    AuthError::new(
        AuthErrorKind::Network,
        "Branchcraft could not reach GitHub.",
        Some(detail.to_owned()),
    )
}

fn map_storage(error: StorageError) -> AuthError {
    let detail = match error {
        StorageError::Unavailable => "No compatible system keyring is available.",
        StorageError::AccessDenied => "Unlock the system keyring and try again.",
        StorageError::Corrupt => "The saved session is damaged. Sign out and sign in again.",
        StorageError::Unknown => "The system keyring operation failed.",
    };
    AuthError::new(
        AuthErrorKind::CredentialStore,
        "Branchcraft could not access the secure credential store.",
        Some(detail.to_owned()),
    )
}

fn map_github(error: GitHubError) -> AuthError {
    match error {
        GitHubError::Unauthorized => expired_error(),
        GitHubError::Forbidden => AuthError::new(
            AuthErrorKind::AuthorizationDenied,
            "GitHub denied access to the account.",
            Some("Review the OAuth authorization and try again.".to_owned()),
        ),
        GitHubError::Network => network_error("Check your network connection and try again."),
        GitHubError::InvalidResponse | GitHubError::Api => AuthError::new(
            AuthErrorKind::InvalidResponse,
            "GitHub returned an unexpected response.",
            Some("Try again in a moment.".to_owned()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use branchcraft_test_support::MemorySecretStore;
    use std::{
        io::{Read, Write},
        net::TcpStream as StdTcpStream,
        sync::{
            Mutex as StdMutex,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
    };

    #[derive(Clone, Copy)]
    enum CallbackMode {
        Valid,
        WrongState,
        Never,
    }

    struct FakeBrowser {
        mode: CallbackMode,
        opened: StdMutex<Option<Url>>,
    }

    impl FakeBrowser {
        fn new(mode: CallbackMode) -> Self {
            Self {
                mode,
                opened: StdMutex::new(None),
            }
        }
    }

    impl BrowserLauncher for FakeBrowser {
        fn open(&self, url: &Url) -> Result<(), AuthError> {
            *self.opened.lock().expect("browser lock") = Some(url.clone());
            if matches!(self.mode, CallbackMode::Never) {
                return Ok(());
            }
            let redirect = url
                .query_pairs()
                .find(|(name, _)| name == "redirect_uri")
                .map(|(_, value)| value.into_owned())
                .expect("redirect uri");
            let state = url
                .query_pairs()
                .find(|(name, _)| name == "state")
                .map(|(_, value)| value.into_owned())
                .expect("state");
            let mode = self.mode;
            thread::spawn(move || {
                let redirect = Url::parse(&redirect).expect("parse redirect");
                let callback_state = if matches!(mode, CallbackMode::WrongState) {
                    "unexpected"
                } else {
                    &state
                };
                let mut stream = StdTcpStream::connect((
                    redirect.host_str().expect("callback host"),
                    redirect.port().expect("callback port"),
                ))
                .expect("connect callback");
                write!(
                    stream,
                    "GET {}?code=test-code&state={} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
                    redirect.path(),
                    callback_state
                )
                .expect("send callback");
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .expect("read callback response");
            });
            Ok(())
        }
    }

    struct FakeOAuth {
        refreshes: AtomicUsize,
    }

    #[async_trait]
    impl OAuthProvider for FakeOAuth {
        fn authorization_url(
            &self,
            redirect_uri: &Url,
            state: &str,
            code_challenge: &str,
        ) -> Result<Url, AuthError> {
            let mut url = Url::parse(GITHUB_AUTHORIZE_URL).expect("authorize url");
            url.query_pairs_mut()
                .append_pair("redirect_uri", redirect_uri.as_str())
                .append_pair("state", state)
                .append_pair("code_challenge", code_challenge)
                .append_pair("code_challenge_method", "S256");
            Ok(url)
        }

        async fn exchange_code(
            &self,
            code: &SecretString,
            _redirect_uri: &Url,
            code_verifier: &SecretString,
        ) -> Result<OAuthTokenSet, AuthError> {
            assert_eq!(code.expose_secret(), "test-code");
            assert_eq!(code_verifier.expose_secret().len(), 43);
            Ok(test_tokens(3600))
        }

        async fn refresh(&self, _refresh_token: &SecretString) -> Result<OAuthTokenSet, AuthError> {
            self.refreshes.fetch_add(1, Ordering::Relaxed);
            Ok(test_tokens(3600))
        }
    }

    struct FakeGitHub;

    #[async_trait]
    impl GitHubService for FakeGitHub {
        async fn current_user(&self, access_token: &SecretString) -> Result<Account, GitHubError> {
            assert_eq!(access_token.expose_secret(), "access-token");
            Ok(test_account())
        }
    }

    fn test_tokens(expires_in: u64) -> OAuthTokenSet {
        OAuthTokenSet {
            access_token: SecretString::from("access-token"),
            refresh_token: Some(SecretString::from("refresh-token")),
            expires_in: Some(expires_in),
            refresh_token_expires_in: Some(7200),
        }
    }

    fn test_account() -> Account {
        Account {
            id: 7,
            login: "branchcrafter".to_owned(),
            name: Some("Branch Crafter".to_owned()),
            avatar_url: "https://avatars.githubusercontent.com/u/7".to_owned(),
            profile_url: "https://github.com/branchcrafter".to_owned(),
        }
    }

    fn service(mode: CallbackMode) -> (BranchcraftAuth, Arc<MemorySecretStore>, Arc<FakeBrowser>) {
        let storage = Arc::new(MemorySecretStore::default());
        let browser = Arc::new(FakeBrowser::new(mode));
        let auth = BranchcraftAuth::with_dependencies(
            true,
            Some(Arc::new(FakeOAuth {
                refreshes: AtomicUsize::new(0),
            })),
            Arc::new(FakeGitHub),
            storage.clone(),
            browser.clone(),
        );
        (auth, storage, browser)
    }

    #[tokio::test]
    async fn unconfigured_build_reports_without_opening_a_browser() {
        let auth = BranchcraftAuth::with_dependencies(
            false,
            None,
            Arc::new(FakeGitHub),
            Arc::new(MemorySecretStore::default()),
            Arc::new(FakeBrowser::new(CallbackMode::Never)),
        );

        assert_eq!(
            auth.status().await.expect("auth status"),
            AuthStatus::Unconfigured
        );
        assert_eq!(
            auth.login()
                .await
                .expect_err("login must be unavailable")
                .kind,
            AuthErrorKind::NotConfigured
        );
    }

    #[tokio::test]
    async fn browser_login_uses_pkce_and_persists_only_in_the_secret_store() {
        let (auth, storage, browser) = service(CallbackMode::Valid);

        let account = auth.login().await.expect("login");

        assert_eq!(account.login, "branchcrafter");
        assert!(storage.contains(GITHUB_CREDENTIAL_KEY));
        let opened = browser
            .opened
            .lock()
            .expect("browser lock")
            .clone()
            .expect("authorization URL");
        let parameters = opened.query_pairs().collect::<Vec<_>>();
        assert!(
            parameters
                .iter()
                .any(|(name, value)| { name == "code_challenge_method" && value == "S256" })
        );
        assert!(
            parameters
                .iter()
                .any(|(name, value)| { name == "code_challenge" && value.len() == 43 })
        );
        assert!(matches!(
            auth.status().await.expect("signed-in status"),
            AuthStatus::SignedIn { .. }
        ));

        auth.logout().await.expect("logout");
        assert!(!storage.contains(GITHUB_CREDENTIAL_KEY));
        assert_eq!(
            auth.status().await.expect("signed-out status"),
            AuthStatus::SignedOut
        );
    }

    #[tokio::test]
    async fn rejects_a_callback_with_the_wrong_state() {
        let (auth, storage, _) = service(CallbackMode::WrongState);

        let error = auth.login().await.expect_err("state mismatch");

        assert_eq!(error.kind, AuthErrorKind::StateMismatch);
        assert!(!storage.contains(GITHUB_CREDENTIAL_KEY));
    }

    #[tokio::test]
    async fn pending_login_can_be_cancelled() {
        let (auth, _, _) = service(CallbackMode::Never);
        let auth = Arc::new(auth);
        let pending = {
            let auth = Arc::clone(&auth);
            tokio::spawn(async move { auth.login().await })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        auth.cancel_login().await;

        let error = pending
            .await
            .expect("login task")
            .expect_err("cancelled login");
        assert_eq!(error.kind, AuthErrorKind::Cancelled);
    }
}
