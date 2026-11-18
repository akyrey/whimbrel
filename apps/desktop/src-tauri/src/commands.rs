//! Tauri commands: thin wrappers over functions that take the state, so tests call them
//! without a running app. Each is logged by name, duration and error code only.

use tauri::State;
use wbl_http::Response;

use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::{RequestCancelArgs, RequestSendArgs};
use crate::logging::logged;
use crate::resolve::{masked, resolved, scopes};
use crate::secrets::Secrets;
use crate::state::AppState;

/// The request as it will be sent, variables resolved.
fn prepare(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
) -> Result<wbl_core::interpolate::Resolved, AppError> {
    let root = args
        .root
        .as_deref()
        .map(|r| collections.opened(r))
        .transpose()?;
    let scopes = scopes(
        root.as_deref(),
        args.environment.as_deref(),
        args.example.as_ref(),
        secrets,
    )?;
    resolved(&args.request, args.example.as_ref(), &scopes)
}

/// Sends a request; a newer one from the same tab cancels it.
pub async fn send(
    state: &AppState,
    collections: &Collections,
    secrets: &Secrets,
    args: RequestSendArgs,
) -> Result<Response, AppError> {
    let request = prepare(collections, secrets, &args)?.request;
    let ticket = state.in_flight.start(&args.request_id)?;
    let result = state.sender.send(&request, &ticket.token).await;
    state.in_flight.finish(&args.request_id, &ticket)?;
    Ok(result?)
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
) -> Result<Response, AppError> {
    logged("request_send", send(&state, &collections, &secrets, args)).await
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
