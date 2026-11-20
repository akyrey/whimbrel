//! OAuth 2 tokens (FR-5.2): kept in the keychain per collection and configuration, refreshed
//! when stale, fetched by the flow the configuration names. Several named tokens can be kept;
//! one is in use. A token never reaches the frontend; only its name and state do.

use tauri::{AppHandle, State};
use wbl_auth::DeviceCode;

use crate::auth_apply::AuthContext;
use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::RequestSendArgs;
use crate::logging::logged;
use crate::secrets::Secrets;
use crate::state::AppState;
pub use wbl_engine::oauth::{
    MAX_NAME_CHARS, OAuthStatus, OAuthToken, TokenChange, cached, change, config_of, status, usable,
};

/// Gets a token by the configuration's grant; see `wbl_engine::oauth::authorize`.
pub async fn authorize(
    client: &wbl_auth::OAuthClient,
    secrets: &Secrets,
    context: &AuthContext,
    name: Option<&str>,
    cancel: &tokio_util::sync::CancellationToken,
    open_browser: impl FnOnce(&str) -> Result<(), String>,
    on_device: impl FnOnce(&DeviceCode),
) -> Result<OAuthStatus, AppError> {
    Ok(wbl_engine::oauth::authorize(
        client,
        secrets,
        context,
        name,
        cancel,
        open_browser,
        on_device,
    )
    .await?)
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
        Ok(status(
            &secrets,
            &context_for(&collections, &secrets, &args)?,
        )?)
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
        Ok(self::change(
            &secrets,
            &context_for(&collections, &secrets, &args)?,
            &change,
        )?)
    })
    .await
}

#[cfg(test)]
mod tests;
