//! A collection on disk: `collection.toml` at the root, folders as directories, requests as
//! `*.req.toml` (FILE_FORMAT.md). Paths crossing to the frontend are relative, with `/`.

pub mod environments;
mod files;
mod ops;
mod paths;

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::{FormatError, RequestFile};
pub use files::CollectionInfo;
pub use ops::{
    ReadRequest, create, create_folder, create_request, delete, duplicate_request, move_request,
    read_request, rename_request, save_request, set_order,
};
pub use paths::hash;

/// The collection's id (keys its keychain entries), without reading its tree.
pub fn id(root: &Path) -> Result<String, FormatError> {
    Ok(files::read_collection(&fs::read_to_string(root.join(COLLECTION_FILE))?)?.id)
}

/// The collection's own `[variables]`, the lowest scope.
pub fn variables(root: &Path) -> Result<std::collections::BTreeMap<String, String>, FormatError> {
    let text = fs::read_to_string(root.join(COLLECTION_FILE))?;
    let doc = files::parse_doc(&text)?;
    crate::fields::string_map(doc.as_table(), "variables")
}

pub fn save_variables(
    root: &Path,
    variables: &std::collections::BTreeMap<String, String>,
) -> Result<(), FormatError> {
    let marker = root.join(COLLECTION_FILE);
    let text = fs::read_to_string(&marker)?;
    let mut doc = files::parse_doc(&text)?;
    crate::fields::set_string_map(doc.as_table_mut(), "variables", variables);
    paths::write_atomic(&marker, &doc.to_string())
}
use paths::{COLLECTION_FILE, FOLDER_FILE, REQUEST_SUFFIX, RESERVED_DIRS, relative};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FolderNode {
    pub path: String,
    pub name: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub order: i64,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RequestNode {
    pub path: String,
    pub name: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub order: i64,
    pub method: String,
}

/// A file that could not be read: listed so the user can fix it, never rewritten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrokenNode {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Node {
    Folder(FolderNode),
    Request(RequestNode),
    Broken(BrokenNode),
}

impl Node {
    fn sort_key(&self) -> (i64, String) {
        match self {
            Self::Folder(f) => (f.order, f.name.to_lowercase()),
            Self::Request(r) => (r.order, r.name.to_lowercase()),
            Self::Broken(b) => (0, b.path.to_lowercase()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub children: Vec<Node>,
}

/// Reads the collection whose `collection.toml` is in `root`.
pub fn load(root: &Path) -> Result<Collection, FormatError> {
    let marker = root.join(COLLECTION_FILE);
    if !marker.is_file() {
        return Err(FormatError::Missing(COLLECTION_FILE.to_owned()));
    }
    let info = files::read_collection(&fs::read_to_string(&marker)?)?;
    Ok(Collection {
        id: info.id,
        name: info.name,
        children: children(root, root)?,
    })
}

fn children(root: &Path, dir: &Path) -> Result<Vec<Node>, FormatError> {
    let mut nodes = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if dir == root && RESERVED_DIRS.contains(&name.as_str()) {
                continue;
            }
            nodes.push(folder(root, &path, &name)?);
        } else if name.ends_with(REQUEST_SUFFIX) {
            nodes.push(request(root, &path));
        }
    }
    nodes.sort_by_key(Node::sort_key);
    Ok(nodes)
}

fn folder(root: &Path, dir: &Path, dir_name: &str) -> Result<Node, FormatError> {
    let path = relative(root, dir);
    let marker = dir.join(FOLDER_FILE);
    let info = if marker.is_file() {
        match files::read_folder(&fs::read_to_string(&marker)?, dir_name) {
            Ok(info) => info,
            Err(error) => {
                let path = relative(root, &marker);
                return Ok(Node::Broken(BrokenNode {
                    path,
                    error: error.to_string(),
                }));
            }
        }
    } else {
        files::FolderInfo {
            name: dir_name.to_owned(),
            order: 0,
        }
    };
    Ok(Node::Folder(FolderNode {
        path,
        name: info.name,
        order: info.order,
        children: children(root, dir)?,
    }))
}

fn request(root: &Path, file: &Path) -> Node {
    let path = relative(root, file);
    let read = fs::read_to_string(file)
        .map_err(FormatError::from)
        .and_then(|text| RequestFile::parse(&text))
        .and_then(|parsed| parsed.read());
    match read {
        Ok(stored) => Node::Request(RequestNode {
            path,
            name: stored.name,
            order: stored.order,
            method: stored.request.method,
        }),
        Err(error) => Node::Broken(BrokenNode {
            path,
            error: error.to_string(),
        }),
    }
}
