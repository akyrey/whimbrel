//! OAuth 2 tokens (FR-5.2): cached in the keychain per collection and configuration, refreshed
//! when stale, fetched by the flow the configuration names. The token never reaches the
//! frontend; only its state does.

use serde::Serialize;
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

/// The keychain "environment" slot that holds tokens.
const SLOT: &str = "oauth";

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

pub fn cached(secrets: &Secrets, context: &AuthContext) -> Result<Option<Token>, AppError> {
    let Some(config) = config_of(context) else {
        return Ok(None);
    };
    let Some(text) = secrets.get(&context.collection_id, SLOT, &config.cache_key())? else {
        return Ok(None);
    };
    // An unreadable entry (an older format) is as good as none: get a new token.
    Ok(serde_json::from_str(&text).ok())
}

fn store(
    secrets: &Secrets,
    context: &AuthContext,
    config: &OAuth2Config,
    token: &Token,
) -> Result<(), AppError> {
    let text = serde_json::to_string(token).map_err(|e| AppError::internal(e.to_string()))?;
    secrets.set(&context.collection_id, SLOT, &config.cache_key(), &text)
}

pub fn forget(secrets: &Secrets, context: &AuthContext) -> Result<(), AppError> {
    match config_of(context) {
        Some(config) => secrets.clear(&context.collection_id, SLOT, &config.cache_key()),
        None => Ok(()),
    }
}

/// A token to send with: the cached one, refreshed when stale. `None`: get one first.
pub async fn usable(
    client: &OAuthClient,
    secrets: &Secrets,
    context: &AuthContext,
) -> Result<Option<Token>, AppError> {
    let Some(config) = config_of(context) else {
        return Ok(None);
    };
    let Some(token) = cached(secrets, context)? else {
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
            store(secrets, context, config, &fresh)?;
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
}

pub fn status(secrets: &Secrets, context: &AuthContext) -> Result<OAuthStatus, AppError> {
    let token = cached(secrets, context)?;
    Ok(OAuthStatus {
        configured: config_of(context).is_some(),
        grant: config_of(context).map(|c| c.grant),
        has_token: token.is_some(),
        expires_at_ms: token.as_ref().and_then(|t| t.expires_at_ms),
        scope: token.as_ref().and_then(|t| t.scope.clone()),
        can_refresh: token.as_ref().is_some_and(|t| t.refresh_token.is_some()),
    })
}

/// Gets a token by the configuration's grant and caches it. `open_browser` shows the
/// authorization page; `on_device` shows the device code while polling.
pub async fn authorize(
    client: &OAuthClient,
    secrets: &Secrets,
    context: &AuthContext,
    cancel: &tokio_util::sync::CancellationToken,
    open_browser: impl FnOnce(&str) -> Result<(), String>,
    on_device: impl FnOnce(&DeviceCode),
) -> Result<OAuthStatus, AppError> {
    let config = config_of(context)
        .ok_or_else(|| AppError::new("invalid_argument", "This request does not use OAuth 2"))?;
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
    store(secrets, context, config, &token)?;
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
            // A closed window cannot be told; polling goes on.
            let _ = on_device.send(device.clone());
        };
        let result = authorize(&state.oauth, &secrets, &context, &ticket.token, open, show).await;
        state.in_flight.finish(&id, &ticket)?;
        result
    })
    .await
}

#[tauri::command]
pub async fn oauth_forget(
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestSendArgs,
) -> Result<(), AppError> {
    logged("oauth_forget", async {
        forget(&secrets, &context_for(&collections, &secrets, &args)?)
    })
    .await
}

#[cfg(test)]
mod tests;
