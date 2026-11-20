//! Tauri commands: thin wrappers over functions that take the state, so tests call them
//! without a running app. Each is logged by name, duration and error code only.

use tauri::State;
use wbl_engine::send::Sources;
use wbl_http::StreamEvent;

use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::{RequestCancelArgs, RequestSendArgs};
use crate::logging::logged;
use crate::resolve::masked;
use crate::secrets::Secrets;
use crate::state::AppState;

pub use wbl_engine::runtime::runtime_key;
pub use wbl_engine::send::Parts;

/// The request's collection, which must be open.
fn root_of(
    collections: &Collections,
    args: &RequestSendArgs,
) -> Result<Option<std::path::PathBuf>, AppError> {
    args.root
        .as_deref()
        .map(|r| collections.opened(r))
        .transpose()
}

fn sources<'a>(
    collections: &'a Collections,
    secrets: &'a Secrets,
    root: Option<&'a std::path::Path>,
) -> Sources<'a> {
    Sources {
        root,
        runtime: &collections.runtime,
        secrets,
    }
}

pub fn parts(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
) -> Result<Parts, AppError> {
    let root = root_of(collections, args)?;
    Ok(wbl_engine::send::parts(
        &sources(collections, secrets, root.as_deref()),
        args,
    )?)
}

/// Sends a request; a newer one from the same tab cancels it. Stream events go to `on_event`.
pub async fn send(
    state: &AppState,
    collections: &Collections,
    secrets: &Secrets,
    on_event: &mut (dyn FnMut(StreamEvent) + Send),
    args: RequestSendArgs,
) -> Result<crate::checks::Sent, AppError> {
    let root = root_of(collections, &args)?;
    let sources = sources(collections, secrets, root.as_deref());
    let ready = wbl_engine::send::ready(&state.oauth, &sources, &args).await?;
    let sent_at_ms = crate::history::now_ms();
    let ticket = state.in_flight.start(&args.request_id)?;
    let result = state
        .sender
        .send_with(
            &ready.resolved.request,
            &ready.exchange,
            &ticket.token,
            on_event,
        )
        .await
        .map_err(AppError::from);
    state.in_flight.finish(&args.request_id, &ticket)?;
    let resolved = &ready.resolved;
    crate::history::record(
        state,
        &wbl_store::NewEntry {
            sent_at_ms,
            collection: args.root.clone(),
            name: args
                .name
                .clone()
                .unwrap_or_else(|| resolved.request.url.clone()),
            request: masked(resolved),
            outcome: crate::history::outcome(&result, &resolved.secrets),
        },
    );
    let response = result?;
    let checks = wbl_engine::checks::after_send(
        &collections.runtime,
        secrets,
        root.as_deref(),
        &args,
        &response,
    )?;
    Ok(crate::checks::Sent { response, checks })
}

/// The variables a request can use, for completion and hints (FR-2.4).
pub fn variables(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
) -> Result<Vec<crate::resolve::ScopeVariable>, AppError> {
    let root = root_of(collections, args)?;
    Ok(wbl_engine::send::variables(
        &sources(collections, secrets, root.as_deref()),
        args,
    )?)
}

#[tauri::command]
pub async fn variables_in_scope(
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestSendArgs,
) -> Result<Vec<crate::resolve::ScopeVariable>, AppError> {
    logged("variables_in_scope", async {
        variables(&collections, &secrets, &args)
    })
    .await
}

/// What would be sent, secrets masked (FR-2.5).
pub fn preview(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
) -> Result<wbl_core::Request, AppError> {
    let root = root_of(collections, args)?;
    Ok(wbl_engine::send::preview(
        &sources(collections, secrets, root.as_deref()),
        args,
    )?)
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
) -> Result<crate::checks::Sent, AppError> {
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
    let root = root_of(collections, &args.send)?;
    Ok(wbl_engine::send::curl(
        &sources(collections, secrets, root.as_deref()),
        &args.send,
        args.reveal,
    )?)
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
