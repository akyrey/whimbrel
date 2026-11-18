//! `collection.toml` (the root marker) and `_folder.toml` (a folder's title and order).

use toml_edit::DocumentMut;

use crate::fields::{optional_integer, set, string};
use crate::{FORMAT, FormatError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionInfo {
    /// Stable; keys keychain entries (Phase 3). Never regenerated.
    pub id: String,
    pub name: String,
}

pub fn parse_doc(text: &str) -> Result<DocumentMut, FormatError> {
    text.parse::<DocumentMut>()
        .map_err(|e| FormatError::syntax(&e))
}

pub fn read_collection(text: &str) -> Result<CollectionInfo, FormatError> {
    let doc = parse_doc(text)?;
    let root = doc.as_table();
    let format = optional_integer(root, "format", "format")?.unwrap_or(FORMAT);
    if format > FORMAT {
        return Err(FormatError::TooNew {
            found: format,
            supported: FORMAT,
        });
    }
    Ok(CollectionInfo {
        id: string(root, "id", "id")?,
        name: string(root, "name", "name")?,
    })
}

pub fn new_collection(info: &CollectionInfo) -> String {
    let mut doc = DocumentMut::new();
    let root = doc.as_table_mut();
    set(root, "format", FORMAT);
    set(root, "id", info.id.as_str());
    set(root, "name", info.name.as_str());
    doc.to_string()
}

/// A folder's title and order; a folder without `_folder.toml` uses its directory name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderInfo {
    pub name: String,
    pub order: i64,
}

pub fn read_folder(text: &str, fallback_name: &str) -> Result<FolderInfo, FormatError> {
    let doc = parse_doc(text)?;
    let root = doc.as_table();
    let name = match root.get("name") {
        Some(_) => string(root, "name", "name")?,
        None => fallback_name.to_owned(),
    };
    Ok(FolderInfo {
        name,
        order: optional_integer(root, "order", "order")?.unwrap_or(0),
    })
}

/// `text` (an existing `_folder.toml`, or empty) with `info` written in, keeping the rest.
pub fn write_folder(text: &str, info: &FolderInfo) -> Result<String, FormatError> {
    let mut doc = parse_doc(text)?;
    let root = doc.as_table_mut();
    if !root.contains_key("format") {
        set(root, "format", FORMAT);
    }
    set(root, "name", info.name.as_str());
    set(root, "order", info.order);
    Ok(doc.to_string())
}
