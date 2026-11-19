//! OpenAPI import (FR-6) and the spec browser: reading a spec from a file or a URL, a summary
//! before importing, the import itself, and the specs kept in a collection.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;
use url::Url;
use wbl_core::{Body, Header, Request};
use wbl_format::collection;
use wbl_http::Sender;
use wbl_openapi::{Api, Installed, Loader, OpenApiError, Source, StoredSpec};

use crate::collections::Collections;
use crate::error::AppError;
use crate::logging::logged;
use crate::state::AppState;

const ACCEPT: &str =
    "application/vnd.oai.openapi+json, application/json, application/yaml, text/yaml, */*;q=0.5";

impl From<OpenApiError> for AppError {
    fn from(error: OpenApiError) -> Self {
        AppError::new(error.code(), error.to_string())
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SpecSource {
    File { path: String },
    Url { url: String },
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecReadArgs {
    pub source: SpecSource,
    /// Fetch documents the spec refers to on other servers.
    pub allow_remote: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ImportTarget {
    /// A new collection in `folder` (an existing directory).
    New { folder: String, name: String },
    /// A new top-level folder `folder_name` in the open collection `root`.
    Existing { root: String, folder_name: String },
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecImportArgs {
    pub source: SpecSource,
    pub allow_remote: bool,
    pub target: ImportTarget,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecOpenArgs {
    pub root: String,
    /// `openapi/<name>`.
    pub dir: String,
}

/// What an import would make, shown before it is made.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecSummary {
    pub title: String,
    pub api_version: String,
    pub version_text: String,
    #[ts(type = "number")]
    pub operations: usize,
    #[ts(type = "number")]
    pub webhooks: usize,
    /// Top-level folders.
    pub folders: Vec<String>,
    pub environments: Vec<String>,
    pub warnings: Vec<String>,
    /// Documents on other servers it refers to that were not fetched.
    pub remote: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecImported {
    /// The collection imported into.
    pub root: String,
    /// The folder the requests are in (`""`: the top).
    pub folder: String,
    pub installed: Installed,
}

/// Reads `file:` URLs from disk and `http(s):` ones through the app's sender.
pub struct AppLoader<'a> {
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

impl Loader for AppLoader<'_> {
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
) -> Result<(Url, String, String), AppError> {
    match source {
        SpecSource::File { path } => {
            let full = PathBuf::from(path)
                .canonicalize()
                .map_err(|e| AppError::new("not_found", format!("Cannot read {path}: {e}")))?;
            let url = Url::from_file_path(&full)
                .map_err(|()| AppError::new("invalid_path", "Not a file path"))?;
            let text = tokio::fs::read_to_string(&full)
                .await
                .map_err(|e| AppError::new("io", e.to_string()))?;
            Ok((url, text, full.to_string_lossy().into_owned()))
        }
        SpecSource::Url { url } => {
            let parsed = Url::parse(url.trim())
                .ok()
                .filter(|u| matches!(u.scheme(), "http" | "https"))
                .ok_or_else(|| AppError::new("invalid_url", "Enter an http or https URL"))?;
            let text = fetch(sender, &parsed).await?;
            Ok((parsed.clone(), text, parsed.to_string()))
        }
    }
}

#[derive(Debug)]
pub(crate) struct Read {
    pub api: Api,
    pub remote: Vec<String>,
    pub source: Source,
}

pub(crate) async fn read(
    sender: &Sender,
    source: &SpecSource,
    allow_remote: bool,
) -> Result<Read, AppError> {
    let (url, text, location) = source_text(sender, source).await?;
    let loaded = wbl_openapi::load_all(url, &text, &AppLoader { sender }, allow_remote).await?;
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

/// Imports into a new collection or a new folder of an open one.
pub fn install(
    collections: &Collections,
    api: &Api,
    source: &Source,
    target: &ImportTarget,
) -> Result<SpecImported, AppError> {
    let generated = wbl_openapi::generate(api);
    let (root, folder) = match target {
        ImportTarget::New { folder, name } => {
            let root = Path::new(folder).canonicalize().map_err(|e| {
                AppError::new("not_found", format!("Cannot open the folder {folder}: {e}"))
            })?;
            collection::create(&root, name)?;
            (root, String::new())
        }
        ImportTarget::Existing { root, folder_name } => {
            let root = collections.opened(root)?;
            let folder = collection::create_folder(&root, "", folder_name)?;
            (root, folder)
        }
    };
    let installed = wbl_openapi::install(&root, &folder, &generated, source)?;
    Ok(SpecImported {
        root: root.to_string_lossy().into_owned(),
        folder,
        installed,
    })
}

/// The model of a spec kept in a collection. References to other files are read from where
/// the spec was imported from, when that was a file that is still there.
pub async fn open(
    collections: &Collections,
    sender: &Sender,
    args: &SpecOpenArgs,
) -> Result<Api, AppError> {
    let root = collections.opened(&args.root)?;
    let file = wbl_openapi::source_file(&root, &args.dir)
        .ok_or_else(|| AppError::new("not_found", "The spec is not in the collection"))?;
    let text = tokio::fs::read_to_string(&file)
        .await
        .map_err(|e| AppError::new("io", e.to_string()))?;
    let source = wbl_openapi::specs(&root)?
        .into_iter()
        .find(|s| s.dir == args.dir)
        .map(|s| s.source)
        .unwrap_or_default();
    let base = Url::from_file_path(&source)
        .ok()
        .filter(|_| Path::new(&source).is_file())
        .or_else(|| Url::from_file_path(&file).ok())
        .ok_or_else(|| AppError::new("invalid_path", "Not a file path"))?;
    let loaded = wbl_openapi::load_all(base, &text, &AppLoader { sender }, false).await?;
    let mut api = wbl_openapi::build(loaded.documents)?;
    api.warnings.splice(0..0, loaded.warnings);
    Ok(api)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecValidateArgs {
    pub root: String,
    pub dir: String,
    pub operation_id: String,
    pub status: u16,
    #[ts(optional)]
    pub content_type: Option<String>,
    pub body: String,
}

/// A response checked against its operation's schema (FR-7.6).
pub async fn validate(
    collections: &Collections,
    sender: &Sender,
    args: &SpecValidateArgs,
) -> Result<wbl_openapi::Validation, AppError> {
    let api = open(
        collections,
        sender,
        &SpecOpenArgs {
            root: args.root.clone(),
            dir: args.dir.clone(),
        },
    )
    .await?;
    Ok(wbl_openapi::validate(
        &api,
        &args.operation_id,
        args.status,
        args.content_type.as_deref(),
        &args.body,
    ))
}

#[tauri::command]
pub async fn spec_validate(
    state: State<'_, AppState>,
    collections: State<'_, Collections>,
    args: SpecValidateArgs,
) -> Result<wbl_openapi::Validation, AppError> {
    logged(
        "spec_validate",
        validate(&collections, &state.sender, &args),
    )
    .await
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecReimportArgs {
    pub root: String,
    pub dir: String,
    pub source: SpecSource,
    pub allow_remote: bool,
    /// Per operation and field, which side wins a conflict (mine when absent).
    #[ts(optional)]
    pub choices: Option<
        std::collections::BTreeMap<
            String,
            std::collections::BTreeMap<wbl_openapi::Field, wbl_openapi::Choice>,
        >,
    >,
}

/// The kept spec and the new one, for a re-import.
async fn both(
    collections: &Collections,
    sender: &Sender,
    args: &SpecReimportArgs,
) -> Result<(PathBuf, Api, Read), AppError> {
    let root = collections.opened(&args.root)?;
    let old = open(
        collections,
        sender,
        &SpecOpenArgs {
            root: args.root.clone(),
            dir: args.dir.clone(),
        },
    )
    .await?;
    let new = read(sender, &args.source, args.allow_remote).await?;
    Ok((root, old, new))
}

/// What re-importing would change (FR-7.5).
pub async fn reimport_plan(
    collections: &Collections,
    sender: &Sender,
    args: &SpecReimportArgs,
) -> Result<wbl_openapi::Plan, AppError> {
    let (root, old, new) = both(collections, sender, args).await?;
    Ok(wbl_openapi::reimport::plan(
        &root, &args.dir, &old, &new.api,
    )?)
}

pub async fn reimport_apply(
    collections: &Collections,
    sender: &Sender,
    args: &SpecReimportArgs,
) -> Result<wbl_openapi::Plan, AppError> {
    let (root, old, new) = both(collections, sender, args).await?;
    let choices = args.choices.clone().unwrap_or_default();
    Ok(wbl_openapi::reimport::apply(
        &root,
        &args.dir,
        &old,
        &new.api,
        &new.source,
        &choices,
    )?)
}

#[tauri::command]
pub async fn spec_reimport_plan(
    state: State<'_, AppState>,
    collections: State<'_, Collections>,
    args: SpecReimportArgs,
) -> Result<wbl_openapi::Plan, AppError> {
    logged(
        "spec_reimport_plan",
        reimport_plan(&collections, &state.sender, &args),
    )
    .await
}

#[tauri::command]
pub async fn spec_reimport_apply(
    state: State<'_, AppState>,
    collections: State<'_, Collections>,
    args: SpecReimportArgs,
) -> Result<wbl_openapi::Plan, AppError> {
    logged(
        "spec_reimport_apply",
        reimport_apply(&collections, &state.sender, &args),
    )
    .await
}

#[tauri::command]
pub async fn spec_read(
    state: State<'_, AppState>,
    args: SpecReadArgs,
) -> Result<SpecSummary, AppError> {
    logged("spec_read", async {
        let read = read(&state.sender, &args.source, args.allow_remote).await?;
        Ok(summary(&read.api, read.remote))
    })
    .await
}

#[tauri::command]
pub async fn spec_import(
    state: State<'_, AppState>,
    collections: State<'_, Collections>,
    args: SpecImportArgs,
) -> Result<SpecImported, AppError> {
    logged("spec_import", async {
        let read = read(&state.sender, &args.source, args.allow_remote).await?;
        install(&collections, &read.api, &read.source, &args.target)
    })
    .await
}

#[tauri::command]
pub async fn spec_list(
    collections: State<'_, Collections>,
    args: crate::ipc::CollectionArgs,
) -> Result<Vec<StoredSpec>, AppError> {
    logged("spec_list", async {
        Ok(wbl_openapi::specs(&collections.opened(&args.root)?)?)
    })
    .await
}

#[tauri::command]
pub async fn spec_open(
    state: State<'_, AppState>,
    collections: State<'_, Collections>,
    args: SpecOpenArgs,
) -> Result<Api, AppError> {
    logged("spec_open", open(&collections, &state.sender, &args)).await
}

#[cfg(test)]
mod tests;
