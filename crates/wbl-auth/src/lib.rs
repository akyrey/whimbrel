//! OAuth 2 for Whimbrel (FR-5.2): client credentials, authorization code with PKCE through the
//! system browser and a loopback redirect (RFC 8252), device authorization (RFC 8628), refresh.
//! The client authenticates with `client_secret_post` when it has a secret.

mod endpoint;
mod loopback;

use endpoint::{endpoint, unreachable};

use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tokio_util::sync::CancellationToken;
use wbl_core::auth::OAuth2Config;

/// A token is renewed this long before it expires.
const EARLY_MS: i64 = 60_000;
/// Polling slows down by this much when the server asks (RFC 8628 §3.5).
const SLOW_DOWN: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// How the token URL field is named in errors.
const TOKEN_URL: &str = "token URL";
const DEVICE_URL: &str = "device URL";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix milliseconds; `None` when the server did not say.
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub expires_at_ms: Option<i64>,
    pub scope: Option<String>,
}

impl Token {
    /// Usable at `now_ms` (renewed a minute early).
    pub fn is_fresh(&self, now_ms: i64) -> bool {
        self.expires_at_ms.is_none_or(|at| now_ms < at - EARLY_MS)
    }
}

/// What the user needs for the device flow, and what polling needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    #[serde(skip_serializing)]
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub interval_secs: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub expires_in_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FlowError {
    /// The authorization server refused, with its own error code and description.
    #[error("The authorization server said {error}{}", description.as_ref().map(|d| format!(": {d}")).unwrap_or_default())]
    Server {
        error: String,
        description: Option<String>,
    },
    /// A token, authorization or device URL that cannot be sent to.
    #[error("{0}")]
    Endpoint(String),
    #[error("The authorization server could not be reached ({0})")]
    Unreachable(String),
    #[error("The authorization server answered something that is not a token ({0})")]
    Invalid(String),
    #[error("The sign-in was cancelled")]
    Cancelled,
    #[error("The sign-in came back for another request (state mismatch)")]
    StateMismatch,
    #[error("The device code expired before it was confirmed")]
    Expired,
    #[error("{0}")]
    Browser(String),
    /// The redirect cannot be listened for (the registered port is taken).
    #[error("{0}")]
    Redirect(String),
}

impl FlowError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Server { .. } => "oauth_error",
            Self::Endpoint(_) => "oauth_endpoint",
            Self::Unreachable(_) => "oauth_unreachable",
            Self::Invalid(_) => "oauth_invalid",
            Self::Cancelled => "cancelled",
            Self::StateMismatch => "oauth_state_mismatch",
            Self::Expired => "oauth_expired",
            Self::Browser(_) => "oauth_browser",
            Self::Redirect(_) => "oauth_redirect",
        }
    }
}

#[derive(Deserialize)]
struct TokenAnswer {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct ErrorAnswer {
    error: String,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct DeviceAnswer {
    device_code: String,
    user_code: String,
    verification_uri: Option<String>,
    /// Some servers still use the draft's name.
    verification_url: Option<String>,
    verification_uri_complete: Option<String>,
    interval: Option<u64>,
    expires_in: u64,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Random text from the URL-safe alphabet (PKCE verifier, state).
fn random_text(chunks: usize) -> String {
    let bytes: Vec<u8> = (0..chunks)
        .flat_map(|_| uuid::Uuid::new_v4().into_bytes())
        .collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn challenge_of(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(verifier.as_bytes()))
}

pub struct OAuthClient {
    http: reqwest::Client,
}

impl Default for OAuthClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("Whimbrel/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();
        Self { http }
    }

    /// Posts a form to `url` (the `label` field) and reads a JSON answer, or the server's error.
    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        label: &str,
        url: &str,
        config: &OAuth2Config,
        mut form: Vec<(&str, String)>,
    ) -> Result<T, FlowError> {
        form.push(("client_id", config.client_id.clone()));
        if !config.client_secret.is_empty() {
            form.push(("client_secret", config.client_secret.clone()));
        }
        let url = endpoint(label, url)?;
        let response = self
            .http
            .post(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&form)
            .send()
            .await
            .map_err(unreachable)?;
        let ok = response.status().is_success();
        let text = response.text().await.map_err(unreachable)?;
        if ok {
            return serde_json::from_str(&text).map_err(|e| FlowError::Invalid(e.to_string()));
        }
        match serde_json::from_str::<ErrorAnswer>(&text) {
            Ok(e) => Err(FlowError::Server {
                error: e.error,
                description: e.error_description,
            }),
            Err(_) => Err(FlowError::Invalid(format!(
                "an error page of {} characters",
                text.len()
            ))),
        }
    }

    async fn token(
        &self,
        config: &OAuth2Config,
        form: Vec<(&str, String)>,
    ) -> Result<Token, FlowError> {
        let started = now_ms();
        let answer: TokenAnswer = self
            .post(TOKEN_URL, &config.token_url, config, form)
            .await?;
        Ok(Token {
            access_token: answer.access_token,
            refresh_token: answer.refresh_token,
            expires_at_ms: answer.expires_in.map(|secs| started + secs * 1000),
            scope: answer.scope,
        })
    }

    fn scoped<'a>(
        config: &OAuth2Config,
        mut form: Vec<(&'a str, String)>,
    ) -> Vec<(&'a str, String)> {
        if !config.scope.is_empty() {
            form.push(("scope", config.scope.clone()));
        }
        form
    }

    pub async fn client_credentials(&self, config: &OAuth2Config) -> Result<Token, FlowError> {
        let form = Self::scoped(config, vec![("grant_type", "client_credentials".into())]);
        self.token(config, form).await
    }

    pub async fn refresh(
        &self,
        config: &OAuth2Config,
        refresh_token: &str,
    ) -> Result<Token, FlowError> {
        let form = vec![
            ("grant_type", "refresh_token".into()),
            ("refresh_token", refresh_token.to_owned()),
        ];
        let token = self.token(config, form).await?;
        // Servers that do not rotate refresh tokens keep the old one valid.
        Ok(Token {
            refresh_token: token
                .refresh_token
                .or_else(|| Some(refresh_token.to_owned())),
            ..token
        })
    }

    /// Opens `open_browser` on the authorization page, waits for the redirect to a loopback
    /// listener (on the configuration's redirect URI, else a random port), and exchanges the
    /// code (with its PKCE verifier) for a token.
    pub async fn authorization_code(
        &self,
        config: &OAuth2Config,
        cancel: &CancellationToken,
        open_browser: impl FnOnce(&str) -> Result<(), String>,
    ) -> Result<Token, FlowError> {
        let mut url = endpoint("authorization URL", &config.auth_url)?;
        endpoint(TOKEN_URL, &config.token_url)?;
        let listener = loopback::Listener::bind(&config.redirect_uri).await?;
        let redirect_uri = listener.redirect_uri();
        let verifier = random_text(3);
        let state = random_text(1);
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("response_type", "code")
                .append_pair("client_id", &config.client_id)
                .append_pair("redirect_uri", &redirect_uri)
                .append_pair("state", &state)
                .append_pair("code_challenge", &challenge_of(&verifier))
                .append_pair("code_challenge_method", "S256");
            if !config.scope.is_empty() {
                query.append_pair("scope", &config.scope);
            }
        }
        open_browser(url.as_str()).map_err(FlowError::Browser)?;
        let code = tokio::select! {
            () = cancel.cancelled() => return Err(FlowError::Cancelled),
            code = listener.wait(&state) => code?,
        };
        let form = vec![
            ("grant_type", "authorization_code".into()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier),
        ];
        self.token(config, form).await
    }

    /// Starts the device flow: the user enters `user_code` at `verification_uri`.
    pub async fn device_start(&self, config: &OAuth2Config) -> Result<DeviceCode, FlowError> {
        endpoint(DEVICE_URL, &config.device_url)?;
        endpoint(TOKEN_URL, &config.token_url)?;
        let form = Self::scoped(config, Vec::new());
        let answer: DeviceAnswer = self
            .post(DEVICE_URL, &config.device_url, config, form)
            .await?;
        let verification_uri = answer
            .verification_uri
            .or(answer.verification_url)
            .ok_or_else(|| FlowError::Invalid("no verification URI".into()))?;
        Ok(DeviceCode {
            device_code: answer.device_code,
            user_code: answer.user_code,
            verification_uri,
            verification_uri_complete: answer.verification_uri_complete,
            interval_secs: answer.interval.unwrap_or(5),
            expires_in_secs: answer.expires_in,
        })
    }

    /// Polls until the user confirms, the code expires, or `cancel` fires.
    pub async fn device_poll(
        &self,
        config: &OAuth2Config,
        device: &DeviceCode,
        cancel: &CancellationToken,
    ) -> Result<Token, FlowError> {
        let deadline =
            now_ms() + i64::try_from(device.expires_in_secs).unwrap_or(i64::MAX / 2) * 1000;
        let mut interval = Duration::from_secs(device.interval_secs);
        loop {
            let form = vec![
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code".into(),
                ),
                ("device_code", device.device_code.clone()),
            ];
            match self.token(config, form).await {
                Err(FlowError::Server { error, .. }) if error == "authorization_pending" => {}
                Err(FlowError::Server { error, .. }) if error == "slow_down" => {
                    interval += SLOW_DOWN
                }
                other => return other,
            }
            if now_ms() > deadline {
                return Err(FlowError::Expired);
            }
            tokio::select! {
                () = cancel.cancelled() => return Err(FlowError::Cancelled),
                () = tokio::time::sleep(interval) => {}
            }
        }
    }
}
