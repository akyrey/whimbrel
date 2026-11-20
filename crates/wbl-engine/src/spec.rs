//! Reading OpenAPI documents for an import (FR-7): from a file or a URL (through the sender),
//! with the documents they refer to, and a summary of what an import would make. Shared by the
//! app and the MCP server.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use url::Url;
use wbl_core::{Body, Header, Request};
use wbl_http::Sender;
use wbl_openapi::{Api, Loader, OpenApiError, Source};

use crate::error::EngineError;

const ACCEPT: &str =
    "application/vnd.oai.openapi+json, application/json, application/yaml, text/yaml, */*;q=0.5";

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SpecSource {
    File { path: String },
    Url { url: String },
}

/// What an import would make, shown before it is made.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SpecSummary {
    pub title: String,
    pub api_version: String,
    pub version_text: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub operations: usize,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub webhooks: usize,
    /// Top-level folders.
    pub folders: Vec<String>,
    pub environments: Vec<String>,
    pub warnings: Vec<String>,
    /// Documents on other servers it refers to that were not fetched.
    pub remote: Vec<String>,
}

/// Reads `file:` URLs from disk and `http(s):` ones through the app's sender.
pub struct SpecLoader<'a> {
    pub sender: &'a Sender,
}

async fn fetch(sender: &Sender, url: &Url) -> Result<String, OpenApiError> {
    let request = Request {
        method: "GET".into(),
        url: url.to_string(),
        headers: vec![Header {
            name: "Accept".into(),
            value: ACCEPT.into(),
            enabled: true,
        }],
        body: Body::None,
    };
    let response = sender
        .send(&request, &CancellationToken::new())
        .await
        .map_err(|e| OpenApiError::Load(e.message))?;
    if !(200..300).contains(&response.status) {
        return Err(OpenApiError::Load(format!(
            "The server answered {}",
            response.status
        )));
    }
    if response.body.truncated {
        return Err(OpenApiError::Load(
            "The document is too large to read".into(),
        ));
    }
    response
        .body
        .text
        .ok_or_else(|| OpenApiError::Load("The document is not text".into()))
}

impl Loader for SpecLoader<'_> {
    async fn load(&self, url: &Url) -> Result<String, OpenApiError> {
        match url.scheme() {
            "file" => {
                let path = url
                    .to_file_path()
                    .map_err(|()| OpenApiError::Load(format!("Not a file: {url}")))?;
                tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| OpenApiError::Load(e.to_string()))
            }
            "http" | "https" => fetch(self.sender, url).await,
            other => Err(OpenApiError::Load(format!(
                "Cannot read {other}: references"
            ))),
        }
    }
}

/// The source's text and the URL references in it are relative to.
async fn source_text(
    sender: &Sender,
    source: &SpecSource,
) -> Result<(Url, String, String), EngineError> {
    match source {
        SpecSource::File { path } => {
            let full = PathBuf::from(path)
                .canonicalize()
                .map_err(|e| EngineError::new("not_found", format!("Cannot read {path}: {e}")))?;
            let url = Url::from_file_path(&full)
                .map_err(|()| EngineError::new("invalid_path", "Not a file path"))?;
            let text = tokio::fs::read_to_string(&full)
                .await
                .map_err(|e| EngineError::new("io", e.to_string()))?;
            Ok((url, text, full.to_string_lossy().into_owned()))
        }
        SpecSource::Url { url } => {
            let parsed = Url::parse(url.trim())
                .ok()
                .filter(|u| matches!(u.scheme(), "http" | "https"))
                .ok_or_else(|| EngineError::new("invalid_url", "Enter an http or https URL"))?;
            let text = fetch(sender, &parsed).await?;
            Ok((parsed.clone(), text, parsed.to_string()))
        }
    }
}

#[derive(Debug)]
pub struct Read {
    pub api: Api,
    pub remote: Vec<String>,
    pub source: Source,
}

pub async fn read(
    sender: &Sender,
    source: &SpecSource,
    allow_remote: bool,
) -> Result<Read, EngineError> {
    let (url, text, location) = source_text(sender, source).await?;
    let loaded = wbl_openapi::load_all(url, &text, &SpecLoader { sender }, allow_remote).await?;
    let mut api = wbl_openapi::build(loaded.documents)?;
    api.warnings.splice(0..0, loaded.warnings);
    Ok(Read {
        api,
        remote: loaded.remote,
        source: Source { location, text },
    })
}

pub fn summary(api: &Api, remote: Vec<String>) -> SpecSummary {
    let generated = wbl_openapi::generate(api);
    let mut folders: Vec<String> = generated
        .folders
        .iter()
        .filter(|f| f.len() == 1)
        .map(|f| f[0].clone())
        .collect();
    if generated.requests.iter().any(|r| r.folder.is_empty()) {
        folders.push("(top level)".into());
    }
    SpecSummary {
        title: api.title.clone(),
        api_version: api.api_version.clone(),
        version_text: api.version_text.clone(),
        operations: api.operations.len(),
        webhooks: api.webhooks.len(),
        folders,
        environments: generated
            .environments
            .iter()
            .map(|e| e.name.clone())
            .collect(),
        warnings: generated.warnings,
        remote,
    }
}

/// The model of a spec kept in a collection. References to other files are read from where
/// the spec was imported from, when that was a file that is still there.
pub async fn open(sender: &Sender, root: &Path, dir: &str) -> Result<Api, EngineError> {
    let file = wbl_openapi::source_file(root, dir)
        .ok_or_else(|| EngineError::new("not_found", "The spec is not in the collection"))?;
    let text = tokio::fs::read_to_string(&file)
        .await
        .map_err(|e| EngineError::new("io", e.to_string()))?;
    let source = wbl_openapi::specs(root)?
        .into_iter()
        .find(|s| s.dir == dir)
        .map(|s| s.source)
        .unwrap_or_default();
    let base = Url::from_file_path(&source)
        .ok()
        .filter(|_| Path::new(&source).is_file())
        .or_else(|| Url::from_file_path(&file).ok())
        .ok_or_else(|| EngineError::new("invalid_path", "Not a file path"))?;
    let loaded = wbl_openapi::load_all(base, &text, &SpecLoader { sender }, false).await?;
    let mut api = wbl_openapi::build(loaded.documents)?;
    api.warnings.splice(0..0, loaded.warnings);
    Ok(api)
}

#[cfg(test)]
mod tests;
