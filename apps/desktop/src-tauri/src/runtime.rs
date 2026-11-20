//! Runtime variables (FR-6.1, D-H17): what captures took from responses, per collection, kept in
//! memory until the app quits. They come after the request's own variables and before the
//! environment.

use ts_rs::TS;

use crate::error::AppError;

pub use wbl_engine::runtime::{LOOSE, Runtime, RuntimeVariable};

#[derive(Debug, Clone, serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeArgs {
    /// The collection; requests outside one when absent.
    #[ts(optional)]
    pub root: Option<String>,
}

fn key_of(
    collections: &crate::collections::Collections,
    args: &RuntimeArgs,
) -> Result<String, AppError> {
    let root = args
        .root
        .as_deref()
        .map(|r| collections.opened(r))
        .transpose()?;
    Ok(crate::commands::runtime_key(root.as_deref()))
}

pub fn list(
    collections: &crate::collections::Collections,
    args: &RuntimeArgs,
) -> Result<Vec<RuntimeVariable>, AppError> {
    Ok(collections.runtime.list(&key_of(collections, args)?)?)
}

pub fn clear(
    collections: &crate::collections::Collections,
    args: &RuntimeArgs,
) -> Result<(), AppError> {
    Ok(collections.runtime.clear(&key_of(collections, args)?)?)
}

#[tauri::command]
pub async fn runtime_list(
    collections: tauri::State<'_, crate::collections::Collections>,
    args: RuntimeArgs,
) -> Result<Vec<RuntimeVariable>, AppError> {
    crate::logging::logged("runtime_list", async { list(&collections, &args) }).await
}

#[tauri::command]
pub async fn runtime_clear(
    collections: tauri::State<'_, crate::collections::Collections>,
    args: RuntimeArgs,
) -> Result<(), AppError> {
    crate::logging::logged("runtime_clear", async { clear(&collections, &args) }).await
}
