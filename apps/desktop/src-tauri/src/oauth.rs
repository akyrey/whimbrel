//! OAuth 2 tokens (FR-5.2): kept in the keychain per collection and configuration, refreshed
//! when stale, fetched by the flow the configuration names. Several named tokens can be kept;
//! one is in use. A token never reaches the frontend; only its name and state do.

mod tokens;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use ts_rs::TS;
use wbl_auth::{DeviceCode, FlowError, OAuthClient, Token};
use wbl_core::auth::{Auth, Grant, OAuth2Config};

use crate::auth_apply::AuthContext;
use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::RequestSendArgs;
use crate::logging::logged;
use crate::secrets::Secrets;
use crate::state::AppState;
pub use tokens::OAuthToken;
use tokens::{Tokens, clean_name};

impl From<FlowError> for AppError {
    fn from(error: FlowError) -> Self {
        AppError::new(error.code(), error.to_string())
    }
}

/// The OAuth 2 configuration that applies, if any.
pub fn config_of(context: &AuthContext) -> Option<&OAuth2Config> {
    match &context.auth.kind {
        Auth::OAuth2(config) => Some(config),
        _ => None,
    }
}

/// The tokens of the configuration that applies, if any.
fn tokens_of<'a>(secrets: &'a Secrets, context: &'a AuthContext) -> Option<Tokens<'a>> {
    config_of(context).map(|config| Tokens::new(secrets, &context.collection_id, config))
}

fn require_tokens<'a>(
    secrets: &'a Secrets,
    context: &'a AuthContext,
) -> Result<Tokens<'a>, AppError> {
    tokens_of(secrets, context)
        .ok_or_else(|| AppError::new("invalid_argument", "This request does not use OAuth 2"))
}

/// The token in use, as it is (not refreshed).
pub fn cached(secrets: &Secrets, context: &AuthContext) -> Result<Option<Token>, AppError> {
    match tokens_of(secrets, context) {
        Some(tokens) => Ok(tokens.active()?.map(|(_, token)| token)),
        None => Ok(None),
    }
}

/// A token to send with: the cached one, refreshed when stale. `None`: get one first.
pub async fn usable(
    client: &OAuthClient,
    secrets: &Secrets,
    context: &AuthContext,
) -> Result<Option<Token>, AppError> {
    let (Some(config), Some(tokens)) = (config_of(context), tokens_of(secrets, context)) else {
        return Ok(None);
    };
    let Some((id, token)) = tokens.active()? else {
        return Ok(None);
    };
    if token.is_fresh(wbl_auth::now_ms()) {
        return Ok(Some(token));
    }
    let Some(refresh) = token.refresh_token.as_deref() else {
        return Ok(None);
    };
    match client.refresh(config, refresh).await {
        Ok(fresh) => {
            tokens.replace(&id, &fresh)?;
            Ok(Some(fresh))
        }
        // A refused refresh means signing in again.
        Err(FlowError::Server { .. }) => Ok(None),
        Err(other) => Err(other.into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStatus {
    /// The request's effective auth is OAuth 2.
    pub configured: bool,
    pub grant: Option<Grant>,
    pub has_token: bool,
    #[ts(type = "number | null")]
    pub expires_at_ms: Option<i64>,
    pub scope: Option<String>,
    pub can_refresh: bool,
    /// Every token kept for this configuration; the one in use is `active`.
    pub tokens: Vec<OAuthToken>,
}

/// The state of the token in use and the list of kept tokens.
pub fn status(secrets: &Secrets, context: &AuthContext) -> Result<OAuthStatus, AppError> {
    let tokens = match tokens_of(secrets, context) {
        Some(tokens) => tokens.list()?,
        None => Vec::new(),
    };
    let active = tokens.iter().find(|t| t.active);
    Ok(OAuthStatus {
        configured: config_of(context).is_some(),
        grant: config_of(context).map(|c| c.grant),
        has_token: active.is_some(),
        expires_at_ms: active.and_then(|t| t.expires_at_ms),
        scope: active.and_then(|t| t.scope.clone()),
        can_refresh: active.is_some_and(|t| t.can_refresh),
        tokens,
    })
}

/// What the token list can be asked to do.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TokenChange {
    Rename { id: String, name: String },
    Select { id: String },
    Remove { id: String },
}

pub fn change(
    secrets: &Secrets,
    context: &AuthContext,
    change: &TokenChange,
) -> Result<OAuthStatus, AppError> {
    let tokens = require_tokens(secrets, context)?;
    match change {
        TokenChange::Rename { id, name } => tokens.rename(id, name)?,
        TokenChange::Select { id } => tokens.select(id)?,
        TokenChange::Remove { id } => tokens.remove(id)?,
    }
    status(secrets, context)
}

/// Gets a token by the configuration's grant and keeps it under `name` (a default when `None`)
/// as the one in use. `open_browser` shows the authorization page; `on_device` shows the
/// device code while polling.
pub async fn authorize(
    client: &OAuthClient,
    secrets: &Secrets,
    context: &AuthContext,
    name: Option<&str>,
    cancel: &tokio_util::sync::CancellationToken,
    open_browser: impl FnOnce(&str) -> Result<(), String>,
    on_device: impl FnOnce(&DeviceCode),
) -> Result<OAuthStatus, AppError> {
    let config = config_of(context)
        .ok_or_else(|| AppError::new("invalid_argument", "This request does not use OAuth 2"))?;
    let tokens = require_tokens(secrets, context)?;
    // A bad name is refused before the sign-in, not after it.
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(clean_name)
        .transpose()?;
    let token = match config.grant {
        Grant::ClientCredentials => client.client_credentials(config).await?,
        Grant::AuthorizationCode => {
            client
                .authorization_code(config, cancel, open_browser)
                .await?
        }
        Grant::DeviceCode => {
            let device = client.device_start(config).await?;
            on_device(&device);
            client.device_poll(config, &device, cancel).await?
        }
    };
    tokens.add(
        &tokens.index()?,
        name.as_deref(),
        &token,
        wbl_auth::now_ms(),
    )?;
    status(secrets, context)
}

fn context_for(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
) -> Result<AuthContext, AppError> {
    crate::commands::parts(collections, secrets, args).map(|p| p.auth)
}

#[tauri::command]
pub async fn oauth_status(
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestSendArgs,
) -> Result<OAuthStatus, AppError> {
    logged("oauth_status", async {
        status(&secrets, &context_for(&collections, &secrets, &args)?)
    })
    .await
}

/// Runs the sign-in; `request_cancel` with `oauth:<requestId>` stops it.
#[tauri::command]
pub async fn oauth_authorize(
    app: AppHandle,
    state: State<'_, AppState>,
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestSendArgs,
    name: Option<String>,
    on_device: tauri::ipc::Channel<DeviceCode>,
) -> Result<OAuthStatus, AppError> {
    use tauri_plugin_opener::OpenerExt;
    logged("oauth_authorize", async {
        let context = context_for(&collections, &secrets, &args)?;
        let id = format!("oauth:{}", args.request_id);
        let ticket = state.in_flight.start(&id)?;
        let open = |url: &str| {
            app.opener()
                .open_url(url, None::<&str>)
                .map_err(|e| format!("Could not open the browser: {e}"))
        };
        let show = |device: &DeviceCode| {
            // A closed window cannot be told, and a browser that does not open leaves the code
            // on screen: polling goes on either way.
            let _ = on_device.send(device.clone());
            let page = device
                .verification_uri_complete
                .as_deref()
                .unwrap_or(&device.verification_uri);
            let _ = app.opener().open_url(page, None::<&str>);
        };
        let result = authorize(
            &state.oauth,
            &secrets,
            &context,
            name.as_deref(),
            &ticket.token,
            open,
            show,
        )
        .await;
        state.in_flight.finish(&id, &ticket)?;
        result
    })
    .await
}

/// Renames, uses or removes one of the kept tokens of the request's effective auth.
#[tauri::command]
pub async fn oauth_token_change(
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestSendArgs,
    change: TokenChange,
) -> Result<OAuthStatus, AppError> {
    logged("oauth_token_change", async {
        self::change(
            &secrets,
            &context_for(&collections, &secrets, &args)?,
            &change,
        )
    })
    .await
}

#[cfg(test)]
mod tests;
