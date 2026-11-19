//! Writes a [`Generated`] collection into a collection folder, with the spec and the import
//! manifest (`openapi/<name>/source.<ext>`, `import.toml`) kept for re-import (Phase 7).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use toml_edit::{DocumentMut, Item, Table, value};
use wbl_format::FormatError;
use wbl_format::collection::{self, environments, slug};

use crate::generate::Generated;

pub(crate) const SPEC_DIR: &str = "openapi";
pub(crate) const MANIFEST: &str = "import.toml";
const MANIFEST_FORMAT: i64 = 1;

/// The spec as it was read.
#[derive(Debug, Clone)]
pub struct Source {
    /// A URL or a file path, for re-import.
    pub location: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    /// operationId → request file, relative to the collection.
    pub requests: BTreeMap<String, String>,
    /// Ids of the environments created.
    pub environments: Vec<String>,
    /// Where the spec and the manifest went.
    pub spec_dir: String,
    pub warnings: Vec<String>,
}

fn toml_text(text: &str) -> String {
    value(text).to_string().trim().to_owned()
}

/// The request file with its `[openapi]` link to the operation.
fn linked(text: &str, operation_id: &str) -> String {
    format!(
        "{}\n\n[openapi]\noperationId = {}\n",
        text.trim_end(),
        toml_text(operation_id)
    )
}

fn extension(text: &str) -> &'static str {
    if text
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with('{')
    {
        "json"
    } else {
        "yaml"
    }
}

fn create_folders(
    root: &Path,
    base: &str,
    generated: &Generated,
) -> Result<BTreeMap<Vec<String>, String>, FormatError> {
    let mut dirs: BTreeMap<Vec<String>, String> = BTreeMap::new();
    for chain in &generated.folders {
        let (name, parents) = chain
            .split_last()
            .ok_or_else(|| FormatError::Missing("folder name".into()))?;
        let parent = dirs
            .get(parents)
            .cloned()
            .unwrap_or_else(|| base.to_owned());
        dirs.insert(
            chain.clone(),
            collection::create_folder(root, &parent, name)?,
        );
    }
    Ok(dirs)
}

fn spec_dir(root: &Path, name: &str) -> std::path::PathBuf {
    let base = slug(name);
    let mut n = 1;
    loop {
        let candidate = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        let dir = root.join(SPEC_DIR).join(&candidate);
        if !dir.exists() {
            return dir;
        }
        n += 1;
    }
}

fn manifest(
    title: &str,
    source: &Source,
    base: &str,
    files: &BTreeMap<String, (String, String)>,
) -> String {
    let mut doc = DocumentMut::new();
    doc["format"] = value(MANIFEST_FORMAT);
    doc["title"] = value(title);
    doc["source"] = value(source.location.as_str());
    doc["spec_hash"] = value(collection::hash(source.text.as_bytes()));
    doc["folder"] = value(base);
    let mut operations = Table::new();
    for (id, (file, hash)) in files {
        let mut entry = toml_edit::InlineTable::new();
        entry.insert("file", file.as_str().into());
        entry.insert("hash", hash.as_str().into());
        operations.insert(id, value(entry));
    }
    doc.insert("operations", Item::Table(operations));
    doc.to_string()
}

/// Writes `generated` under `base` (`""`: the collection's top) in the collection at `root`.
/// Environments whose name is taken are not created again.
pub fn install(
    root: &Path,
    base: &str,
    generated: &Generated,
    source: &Source,
) -> Result<Installed, FormatError> {
    let dirs = create_folders(root, base, generated)?;
    collection::auth::save_auth(root, base, &generated.auth)?;
    let mut by_folder: BTreeMap<String, Vec<&crate::generate::GeneratedRequest>> = BTreeMap::new();
    for request in &generated.requests {
        let folder = dirs
            .get(&request.folder)
            .cloned()
            .unwrap_or_else(|| base.to_owned());
        by_folder.entry(folder).or_default().push(request);
    }
    let mut files = BTreeMap::new();
    for (folder, requests) in by_folder {
        let stored: Vec<_> = requests.iter().map(|r| r.stored.clone()).collect();
        let written = collection::create_requests(root, &folder, &stored, |i, text| {
            linked(&text, &requests[i].operation_id)
        })?;
        for (request, (path, text)) in requests.iter().zip(written) {
            files.insert(
                request.operation_id.clone(),
                (path, collection::hash(text.as_bytes())),
            );
        }
    }
    let existing: Vec<String> = environments::list(root)?
        .into_iter()
        .filter_map(|e| e.name)
        .collect();
    let mut installed = Installed {
        warnings: generated.warnings.clone(),
        ..Installed::default()
    };
    for environment in &generated.environments {
        if existing.contains(&environment.name) {
            installed.warnings.push(format!(
                "The environment {} exists; set its baseUrl and secrets by hand",
                environment.name
            ));
            continue;
        }
        installed
            .environments
            .push(environments::create(root, environment)?);
    }
    let dir = spec_dir(root, &generated.name);
    fs::create_dir_all(&dir)?;
    collection::write_atomic(
        &dir.join(format!("source.{}", extension(&source.text))),
        &source.text,
    )?;
    collection::write_atomic(
        &dir.join(MANIFEST),
        &manifest(&generated.name, source, base, &files),
    )?;
    installed.spec_dir = dir
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    installed.requests = files
        .into_iter()
        .map(|(id, (path, _))| (id, path))
        .collect();
    Ok(installed)
}
