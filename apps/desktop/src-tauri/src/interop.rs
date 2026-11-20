//! Postman and `.http` files in and out (task 8.4, D-H19).

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;
use wbl_core::auth::{Auth, effective};
use wbl_format::collection::{self, Node};
use wbl_interop::{Imported, Installed, InteropError};

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
pub struct InteropSummary {
    pub name: String,
    #[ts(type = "number")]
    pub requests: usize,
    pub folders: Vec<String>,
    pub environments: Vec<String>,
    #[ts(type = "number")]
    pub variables: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InteropImported {
    pub root: String,
    pub folder: String,
    pub installed: Installed,
}

fn read(path: &str) -> Result<Imported, AppError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| AppError::new("io", format!("Cannot read {path}: {e}")))?;
    Ok(wbl_interop::read(path, &text)?)
}

pub fn summary(path: &str) -> Result<InteropSummary, AppError> {
    let imported = read(path)?;
    Ok(InteropSummary {
        name: imported.name,
        requests: imported.requests.len(),
        folders: imported
            .folders
            .iter()
            .filter(|f| f.len() == 1)
            .map(|f| f[0].clone())
            .collect(),
        environments: imported
            .environments
            .iter()
            .map(|e| e.name.clone())
            .collect(),
        variables: imported.variables.len(),
        warnings: imported.warnings,
    })
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

fn requests_under<'a>(nodes: &'a [Node], folder: &str, inside: bool, out: &mut Vec<&'a str>) {
    for node in nodes {
        match node {
            Node::Request(r) if inside => out.push(&r.path),
            Node::Folder(f) => requests_under(&f.children, folder, inside || f.path == folder, out),
            _ => {}
        }
    }
}

/// Writes the folder's requests (in tree order, each with the auth and headers it inherits)
/// as `.http`.
pub fn export(collections: &Collections, args: &HttpExportArgs) -> Result<usize, AppError> {
    let root = collections.opened(&args.root)?;
    let tree = collection::load(&root)?;
    let mut paths = Vec::new();
    requests_under(
        &tree.children,
        &args.folder,
        args.folder.is_empty(),
        &mut paths,
    );
    let mut requests = Vec::new();
    for path in &paths {
        let stored = collection::read_request(&root, path)?.stored;
        let chain = collection::auth::auth_chain(&root, path)?;
        let inherited = effective(&chain.iter().collect::<Vec<&Auth>>()).clone();
        let headers = collection::headers::headers_chain(&root, path)?;
        let request = wbl_core::inherit::with_inherited(&stored.request, &headers);
        requests.push((wbl_format::StoredRequest { request, ..stored }, inherited));
    }
    std::fs::write(
        &args.destination,
        wbl_interop::http_export::write(&requests),
    )
    .map_err(|e| AppError::new("io", format!("Could not write {}: {e}", args.destination)))?;
    Ok(requests.len())
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
