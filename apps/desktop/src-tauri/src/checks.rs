//! After a send (FR-6): captures and assertions, run by the engine for the collection the
//! request was sent from.

use wbl_http::Response;

use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::{RequestSendArgs, ResponseSelectArgs};
use crate::logging::logged;
use crate::secrets::Secrets;

pub use wbl_engine::checks::{CaptureResult, Checks, Sent, observed};

/// Runs the request's captures and assertions on `response`.
pub fn after_send(
    collections: &Collections,
    secrets: &Secrets,
    args: &RequestSendArgs,
    response: &Response,
) -> Result<Checks, AppError> {
    let root = args
        .root
        .as_deref()
        .map(|r| collections.opened(r))
        .transpose()?;
    Ok(wbl_engine::checks::after_send(
        &collections.runtime,
        secrets,
        root.as_deref(),
        args,
        response,
    )?)
}

/// The values at `args.path` in the body, as one pretty JSON array.
pub fn select(args: &ResponseSelectArgs) -> Result<String, AppError> {
    Ok(wbl_engine::checks::select(&args.body, &args.path)?)
}

#[tauri::command]
pub async fn response_select(args: ResponseSelectArgs) -> Result<String, AppError> {
    logged("response_select", async { select(&args) }).await
}

#[cfg(test)]
mod tests;
