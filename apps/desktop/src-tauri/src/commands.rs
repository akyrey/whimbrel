//! Tauri commands: thin wrappers over functions that take the state, so tests call them
//! without a running app. Each is logged by name, duration and error code only.

use tauri::State;
use wbl_http::{Exchange, Response, StreamEvent};

use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::{RequestCancelArgs, RequestSendArgs};
use crate::logging::logged;
use crate::resolve::{masked, resolved, scopes};
use crate::secrets::Secrets;
use crate::state::AppState;

/// What resolving a request needs: its scopes and its auth.
pub struct Parts {
    pub scopes: Vec<wbl_core::interpolate::Scope>,
    pub auth: crate::auth_apply::AuthContext,
    root: Option<std::path::PathBuf>,
}

pub fn parts(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
) -> Result<Parts, AppError> {
    let root = args
        .root
        .as_deref()
        .map(|r| collections.opened(r))
        .transpose()?;
    let scopes = scopes(
        root.as_deref(),
        args.environment.as_deref(),
        args.example.as_ref(),
        args.variables.as_ref(),
        secrets,
    )?;
    let auth = crate::auth_apply::context(root.as_deref(), args, &scopes)?;
    Ok(Parts { scopes, auth, root })
}

/// The request as it will be sent: example, variables and auth applied.
fn build(
    args: &RequestSendArgs,
    parts: &Parts,
    token: Option<&wbl_auth::Token>,
) -> Result<wbl_core::interpolate::Resolved, AppError> {
    let resolved = resolved(&args.request, args.example.as_ref(), &parts.scopes)?;
    crate::auth_apply::finish(resolved, &parts.auth, token)
}

/// For showing (preview, cURL): the cached token as it is, not refreshed.
fn prepare(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
) -> Result<wbl_core::interpolate::Resolved, AppError> {
    let parts = parts(collections, secrets, args)?;
    let token = crate::oauth::cached(secrets, &parts.auth)?;
    build(args, &parts, token.as_ref())
}

/// The collection's own TLS files, read for each send (they are small, and may change).
fn tls_of(parts: &Parts) -> Result<wbl_http::TlsFiles, AppError> {
    let Some(root) = &parts.root else {
        return Ok(wbl_http::TlsFiles::default());
    };
    let settings = wbl_format::collection::auth::tls(root)?;
    let read = |name: &str| -> Result<Option<Vec<u8>>, AppError> {
        if name.is_empty() {
            return Ok(None);
        }
        let path = wbl_format::collection::auth::tls_file(root, name);
        std::fs::read(&path).map(Some).map_err(|e| {
            AppError::new(
                "tls",
                format!("Cannot read the collection's TLS file {name}: {e}"),
            )
        })
    };
    Ok(wbl_http::TlsFiles {
        ca_pem: read(&settings.ca_file)?,
        client_pem: read(&settings.client_cert_file)?,
    })
}

/// The cookie jar of the collection a request belongs to (by its stable id).
fn jar_of(collections: &Collections, args: &RequestSendArgs) -> Option<String> {
    let root = args
        .root
        .as_deref()
        .and_then(|r| collections.opened(r).ok())?;
    wbl_format::collection::id(&root).ok()
}

/// Sends a request; a newer one from the same tab cancels it. Stream events go to `on_event`.
pub async fn send(
    state: &AppState,
    collections: &Collections,
    secrets: &Secrets,
    on_event: &mut (dyn FnMut(StreamEvent) + Send),
    args: RequestSendArgs,
) -> Result<Response, AppError> {
    let parts = parts(collections, secrets, &args)?;
    let token = crate::oauth::usable(&state.oauth, secrets, &parts.auth).await?;
    let resolved = build(&args, &parts, token.as_ref())?;
    let exchange = Exchange {
        settings: args.settings.unwrap_or_default(),
        jar: jar_of(collections, &args),
        tls: tls_of(&parts)?,
    };
    let sent_at_ms = crate::history::now_ms();
    let ticket = state.in_flight.start(&args.request_id)?;
    let result = state
        .sender
        .send_with(&resolved.request, &exchange, &ticket.token, on_event)
        .await
        .map_err(AppError::from);
    state.in_flight.finish(&args.request_id, &ticket)?;
    crate::history::record(
        state,
        &wbl_store::NewEntry {
            sent_at_ms,
            collection: args.root.clone(),
            name: args
                .name
                .clone()
                .unwrap_or_else(|| resolved.request.url.clone()),
            request: masked(&resolved),
            outcome: crate::history::outcome(&result, &resolved.secrets),
        },
    );
    result
}

/// What would be sent, secrets masked (FR-2.5).
pub fn preview(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
) -> Result<wbl_core::Request, AppError> {
    Ok(masked(&prepare(collections, secrets, args)?))
}

/// Cancels the tab's request; `false` when it had already finished.
pub fn cancel(state: &AppState, args: &RequestCancelArgs) -> Result<bool, AppError> {
    state.in_flight.cancel(&args.request_id)
}

#[tauri::command]
pub async fn request_send(
    state: State<'_, AppState>,
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestSendArgs,
    on_event: tauri::ipc::Channel<StreamEvent>,
) -> Result<Response, AppError> {
    let mut forward = move |event: StreamEvent| {
        // A closed window cannot be told; the response still completes.
        let _ = on_event.send(event);
    };
    logged(
        "request_send",
        send(&state, &collections, &secrets, &mut forward, args),
    )
    .await
}

/// Writes a response body to the file the user chose.
pub fn save_response(
    state: &AppState,
    args: &crate::ipc::ResponseSaveArgs,
) -> Result<(), AppError> {
    use base64::Engine;
    let destination = std::path::Path::new(&args.destination);
    let written = if let Some(file) = &args.file {
        let source = state.spilled(file)?;
        std::fs::copy(source, destination).map(|_| ())
    } else if let Some(encoded) = &args.base64 {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| {
                AppError::new("invalid_argument", format!("The body is not base64: {e}"))
            })?;
        std::fs::write(destination, bytes)
    } else {
        std::fs::write(destination, args.text.as_deref().unwrap_or_default())
    };
    written.map_err(|e| AppError::new("io", format!("Could not write {}: {e}", args.destination)))
}

#[tauri::command]
pub async fn response_save(
    state: State<'_, AppState>,
    args: crate::ipc::ResponseSaveArgs,
) -> Result<(), AppError> {
    logged("response_save", async { save_response(&state, &args) }).await
}

#[tauri::command]
pub async fn request_preview(
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestSendArgs,
) -> Result<wbl_core::Request, AppError> {
    logged("request_preview", async {
        preview(&collections, &secrets, &args)
    })
    .await
}

#[tauri::command]
pub async fn request_cancel(
    state: State<'_, AppState>,
    args: RequestCancelArgs,
) -> Result<bool, AppError> {
    logged("request_cancel", async { cancel(&state, &args) }).await
}

/// The resolved request as a cURL command (FR-2.6).
pub fn curl_command(
    collections: &Collections,
    secrets: &Secrets,
    args: &crate::ipc::CurlExportArgs,
) -> Result<String, AppError> {
    let resolved = prepare(collections, secrets, &args.send)?;
    let request = if args.reveal {
        resolved.request.clone()
    } else {
        masked(&resolved)
    };
    Ok(wbl_core::curl::to_curl(&request))
}

#[tauri::command]
pub async fn curl_export(
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: crate::ipc::CurlExportArgs,
) -> Result<String, AppError> {
    logged("curl_export", async {
        curl_command(&collections, &secrets, &args)
    })
    .await
}

pub fn import_curl(
    args: &crate::ipc::CurlImportArgs,
) -> Result<crate::ipc::CurlImported, AppError> {
    let imported = wbl_core::curl::parse(&args.text)
        .map_err(|e| AppError::new("invalid_curl", e.to_string()))?;
    Ok(crate::ipc::CurlImported {
        request: imported.request,
        settings: imported.settings,
        ignored: imported.ignored,
    })
}

#[tauri::command]
pub async fn curl_import(
    args: crate::ipc::CurlImportArgs,
) -> Result<crate::ipc::CurlImported, AppError> {
    logged("curl_import", async { import_curl(&args) }).await
}

/// The request with the example laid over it ("apply to the editor").
#[tauri::command]
pub async fn example_apply(
    args: crate::ipc::ExampleApplyArgs,
) -> Result<wbl_core::Request, AppError> {
    logged("example_apply", async {
        Ok(wbl_core::example::apply(&args.request, &args.example))
    })
    .await
}

/// The current edits as an example ("save as example"); `invalid_example` when they change the
/// method or path.
#[tauri::command]
pub async fn example_overlay(
    args: crate::ipc::ExampleOverlayArgs,
) -> Result<wbl_core::example::Example, AppError> {
    logged("example_overlay", async {
        wbl_core::example::overlay_of(&args.base, &args.draft, &args.id, &args.name)
            .map_err(|e| AppError::new("invalid_example", e.to_string()))
    })
    .await
}

#[cfg(test)]
mod tests;
