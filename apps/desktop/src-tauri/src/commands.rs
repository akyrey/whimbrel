//! Tauri commands: thin wrappers over functions that take the state, so tests call them
//! without a running app. Each is logged by name, duration and error code only.

use tauri::State;
use wbl_http::Response;

use crate::error::AppError;
use crate::ipc::{RequestCancelArgs, RequestSendArgs};
use crate::logging::logged;
use crate::state::AppState;

/// Sends a request; a newer one from the same tab cancels it.
pub async fn send(state: &AppState, args: RequestSendArgs) -> Result<Response, AppError> {
    let ticket = state.in_flight.start(&args.request_id)?;
    let result = state.sender.send(&args.request, &ticket.token).await;
    state.in_flight.finish(&args.request_id, &ticket)?;
    Ok(result?)
}

/// Cancels the tab's request; `false` when it had already finished.
pub fn cancel(state: &AppState, args: &RequestCancelArgs) -> Result<bool, AppError> {
    state.in_flight.cancel(&args.request_id)
}

#[tauri::command]
pub async fn request_send(
    state: State<'_, AppState>,
    args: RequestSendArgs,
) -> Result<Response, AppError> {
    logged("request_send", send(&state, args)).await
}

#[tauri::command]
pub async fn request_cancel(
    state: State<'_, AppState>,
    args: RequestCancelArgs,
) -> Result<bool, AppError> {
    logged("request_cancel", async { cancel(&state, &args) }).await
}

#[cfg(test)]
mod tests;
