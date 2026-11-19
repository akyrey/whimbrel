//! Specs kept in a collection (`openapi/<name>/`): what was imported and where it went.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::DocumentMut;
use wbl_format::FormatError;

use crate::install::{MANIFEST, SPEC_DIR};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StoredSpec {
    /// `openapi/<name>`, relative to the collection.
    pub dir: String,
    pub title: String,
    /// Where it was imported from.
    pub source: String,
    /// The folder the requests were generated in (`""`: the top).
    pub folder: String,
    /// operationId → request file.
    pub operations: BTreeMap<String, String>,
}

fn text(doc: &DocumentMut, key: &str) -> String {
    doc.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned()
}

fn read(dir: &Path, relative: String) -> Result<StoredSpec, FormatError> {
    let doc: DocumentMut = fs::read_to_string(dir.join(MANIFEST))?
        .parse()
        .map_err(|e| FormatError::syntax(&e))?;
    let operations = doc
        .get("operations")
        .and_then(|o| o.as_table_like())
        .map(|table| {
            table
                .iter()
                .filter_map(|(id, entry)| {
                    let file = entry.as_table_like()?.get("file")?.as_str()?;
                    Some((id.to_owned(), file.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(StoredSpec {
        dir: relative,
        title: text(&doc, "title"),
        source: text(&doc, "source"),
        folder: text(&doc, "folder"),
        operations,
    })
}

/// The specs imported into the collection at `root`; unreadable manifests are skipped.
pub fn specs(root: &Path) -> Result<Vec<StoredSpec>, FormatError> {
    let base = root.join(SPEC_DIR);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&base)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if path.join(MANIFEST).is_file()
            && let Ok(spec) = read(&path, format!("{SPEC_DIR}/{name}"))
        {
            out.push(spec);
        }
    }
    out.sort_by(|a, b| a.title.cmp(&b.title).then_with(|| a.dir.cmp(&b.dir)));
    Ok(out)
}

/// The kept copy of the spec in `dir` (`openapi/<name>`), if there is one.
pub fn source_file(root: &Path, dir: &str) -> Option<PathBuf> {
    let name = dir.strip_prefix(&format!("{SPEC_DIR}/"))?;
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        return None;
    }
    ["yaml", "json"]
        .iter()
        .map(|ext| root.join(SPEC_DIR).join(name).join(format!("source.{ext}")))
        .find(|p| p.is_file())
}

/// One operation of a manifest: its file, the hash of what was generated, and whether the
/// operation has left the spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub file: String,
    pub hash: String,
    pub removed: bool,
}

/// The manifest's operations, with their hashes.
pub(crate) fn entries(root: &Path, dir: &str) -> Result<BTreeMap<String, Entry>, FormatError> {
    let doc: DocumentMut = fs::read_to_string(root.join(dir).join(MANIFEST))?
        .parse()
        .map_err(|e| FormatError::syntax(&e))?;
    let field = |entry: &dyn toml_edit::TableLike, key: &str| {
        entry
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    Ok(doc
        .get("operations")
        .and_then(|o| o.as_table_like())
        .map(|table| {
            table
                .iter()
                .filter_map(|(id, item)| {
                    let entry = item.as_table_like()?;
                    Some((
                        id.to_owned(),
                        Entry {
                            file: field(entry, "file"),
                            hash: field(entry, "hash"),
                            removed: entry
                                .get("removed")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default())
}
