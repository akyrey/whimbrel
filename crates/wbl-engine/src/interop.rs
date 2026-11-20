//! Postman and `.http` files in and out (D-H19): reading one with a summary, and writing a
//! folder as `.http`. Shared by the app and the MCP server.

use std::path::Path;

use serde::Serialize;
use wbl_core::auth::{Auth, effective};
use wbl_format::collection;
use wbl_interop::Imported;

use crate::error::EngineError;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct InteropSummary {
    pub name: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub requests: usize,
    pub folders: Vec<String>,
    pub environments: Vec<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub variables: usize,
    pub warnings: Vec<String>,
}

pub fn read(path: &str) -> Result<Imported, EngineError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| EngineError::new("io", format!("Cannot read {path}: {e}")))?;
    Ok(wbl_interop::read(path, &text)?)
}

pub fn summary(path: &str) -> Result<InteropSummary, EngineError> {
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

/// The folder's requests (`""`: the whole collection) as `.http`, in tree order, each with
/// the auth and headers it inherits; and how many there are.
pub fn export(root: &Path, folder: &str) -> Result<(String, usize), EngineError> {
    let paths = crate::runner::requests_in(root, folder)?;
    let mut requests = Vec::new();
    for path in &paths {
        let stored = collection::read_request(root, path)?.stored;
        let chain = collection::auth::auth_chain(root, path)?;
        let inherited = effective(&chain.iter().collect::<Vec<&Auth>>()).clone();
        let headers = collection::headers::headers_chain(root, path)?;
        let request = wbl_core::inherit::with_inherited(&stored.request, &headers);
        requests.push((wbl_format::StoredRequest { request, ..stored }, inherited));
    }
    Ok((wbl_interop::http_export::write(&requests), requests.len()))
}

#[cfg(test)]
mod tests;
