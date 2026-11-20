//! Postman and `.http` files in and out (task 8.4, D-H19).

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;
use wbl_format::collection;
use wbl_interop::{Imported, Installed, InteropError};

pub use wbl_engine::interop::InteropSummary;

use crate::collections::Collections;
use crate::error::AppError;
use crate::logging::logged;
use crate::spec::ImportTarget;

impl From<InteropError> for AppError {
    fn from(error: InteropError) -> Self {
        AppError::new(error.code(), error.to_string())
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InteropReadArgs {
    /// A Postman collection or environment (`.json`), or a `.http` file.
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InteropImportArgs {
    pub path: String,
    pub target: ImportTarget,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HttpExportArgs {
    pub root: String,
    /// The folder to export (`""`: the whole collection).
    pub folder: String,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InteropImported {
    pub root: String,
    pub folder: String,
    pub installed: Installed,
}

fn read(path: &str) -> Result<Imported, AppError> {
    Ok(wbl_engine::interop::read(path)?)
}

pub fn summary(path: &str) -> Result<InteropSummary, AppError> {
    Ok(wbl_engine::interop::summary(path)?)
}

/// Imports into a new collection or a new folder of an open one. An environment file alone
/// only adds its environment.
pub fn import(
    collections: &Collections,
    args: &InteropImportArgs,
) -> Result<InteropImported, AppError> {
    let imported = read(&args.path)?;
    let (root, folder) = match &args.target {
        ImportTarget::New { folder, name } => {
            let root = Path::new(folder).canonicalize().map_err(|e| {
                AppError::new("not_found", format!("Cannot open the folder {folder}: {e}"))
            })?;
            collection::create(&root, name)?;
            (root, String::new())
        }
        ImportTarget::Existing { root, folder_name } => {
            let root = collections.opened(root)?;
            let folder = if imported.requests.is_empty() {
                String::new()
            } else {
                collection::create_folder(&root, "", folder_name)?
            };
            (root, folder)
        }
    };
    let installed = wbl_interop::install(&root, &folder, &imported)?;
    Ok(InteropImported {
        root: root.to_string_lossy().into_owned(),
        folder,
        installed,
    })
}

/// Writes the folder's requests as `.http`; returns how many.
pub fn export(collections: &Collections, args: &HttpExportArgs) -> Result<usize, AppError> {
    let root = collections.opened(&args.root)?;
    let (text, count) = wbl_engine::interop::export(&root, &args.folder)?;
    std::fs::write(&args.destination, text)
        .map_err(|e| AppError::new("io", format!("Could not write {}: {e}", args.destination)))?;
    Ok(count)
}

#[tauri::command]
pub async fn interop_read(args: InteropReadArgs) -> Result<InteropSummary, AppError> {
    logged("interop_read", async { summary(&args.path) }).await
}

#[tauri::command]
pub async fn interop_import(
    collections: State<'_, Collections>,
    args: InteropImportArgs,
) -> Result<InteropImported, AppError> {
    logged("interop_import", async { import(&collections, &args) }).await
}

#[tauri::command]
pub async fn http_export(
    collections: State<'_, Collections>,
    args: HttpExportArgs,
) -> Result<usize, AppError> {
    logged("http_export", async { export(&collections, &args) }).await
}

#[cfg(test)]
mod tests;
