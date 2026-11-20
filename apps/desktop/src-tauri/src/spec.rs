//! OpenAPI import (FR-6) and the spec browser: reading a spec from a file or a URL, a summary
//! before importing, the import itself, and the specs kept in a collection.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;
use wbl_format::collection;
use wbl_http::Sender;
use wbl_openapi::{Api, Installed, OpenApiError, Source, StoredSpec};

use crate::collections::Collections;
use crate::error::AppError;
use crate::logging::logged;
use crate::state::AppState;

pub use wbl_engine::spec::{Read, SpecSource, SpecSummary, summary};

impl From<OpenApiError> for AppError {
    fn from(error: OpenApiError) -> Self {
        AppError::new(error.code(), error.to_string())
    }
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

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecImported {
    /// The collection imported into.
    pub root: String,
    /// The folder the requests are in (`""`: the top).
    pub folder: String,
    pub installed: Installed,
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

/// The model of a spec kept in a collection.
pub async fn open(
    collections: &Collections,
    sender: &Sender,
    args: &SpecOpenArgs,
) -> Result<Api, AppError> {
    let root = collections.opened(&args.root)?;
    Ok(wbl_engine::spec::open(sender, &root, &args.dir).await?)
}

pub(crate) async fn read(
    sender: &Sender,
    source: &SpecSource,
    allow_remote: bool,
) -> Result<Read, AppError> {
    Ok(wbl_engine::spec::read(sender, source, allow_remote).await?)
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
