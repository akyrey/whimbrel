//! Changing a collection: every write goes through `paths::write_atomic`, every path through
//! `paths::resolve`.

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::children;
use super::files::{self, CollectionInfo, FolderInfo};
use super::paths::{
    COLLECTION_FILE, FOLDER_FILE, REQUEST_SUFFIX, hash, relative, resolve, unique, write_atomic,
};
use crate::{FormatError, RequestFile, StoredRequest};

/// Orders are spaced so that putting something between two neighbours rewrites one file.
const ORDER_STEP: i64 = 10;

/// Creates a collection in `root`: `collection.toml` with a new id, and a `.gitignore` for the
/// per-user overrides. Refuses a folder that already holds one.
pub fn create(root: &Path, name: &str) -> Result<CollectionInfo, FormatError> {
    let marker = root.join(COLLECTION_FILE);
    if marker.exists() {
        return Err(FormatError::Path(
            "This folder already holds a collection".into(),
        ));
    }
    let info = CollectionInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_owned(),
    };
    write_atomic(&marker, &files::new_collection(&info))?;
    let ignore = root.join(".gitignore");
    if !ignore.exists() {
        write_atomic(&ignore, ".local/\n")?;
    }
    Ok(info)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadRequest {
    pub stored: StoredRequest,
    /// Of the file as read; `save_request` refuses to write when the file no longer matches.
    pub hash: String,
}

pub fn read_request(root: &Path, path: &str) -> Result<ReadRequest, FormatError> {
    let text = fs::read_to_string(resolve(root, path, false)?)?;
    let stored = RequestFile::parse(&text)?.read()?;
    Ok(ReadRequest {
        stored,
        hash: hash(text.as_bytes()),
    })
}

/// Writes `stored` into the file at `path` if it is still what was read (`expected_hash`);
/// returns the new hash. A file that does not parse is never written.
pub fn save_request(
    root: &Path,
    path: &str,
    stored: &StoredRequest,
    expected_hash: &str,
) -> Result<String, FormatError> {
    let full = resolve(root, path, false)?;
    let current = fs::read_to_string(&full)?;
    if hash(current.as_bytes()) != expected_hash {
        return Err(FormatError::Conflict);
    }
    let text = RequestFile::parse(&current)?.updated(stored).to_string();
    if text != current {
        write_atomic(&full, &text)?;
    }
    Ok(hash(text.as_bytes()))
}

fn last_order(root: &Path, dir: &Path) -> Result<i64, FormatError> {
    Ok(children(root, dir)?
        .iter()
        .map(|n| n.sort_key().0)
        .max()
        .unwrap_or(0))
}

/// Creates `stored` in `folder` (`""` for the root), after its siblings; returns its path.
pub fn create_request(
    root: &Path,
    folder: &str,
    stored: &StoredRequest,
) -> Result<String, FormatError> {
    let dir = resolve(root, folder, true)?;
    let placed = StoredRequest {
        order: last_order(root, &dir)? + ORDER_STEP,
        ..stored.clone()
    };
    let file = unique(&dir, &stored.name, REQUEST_SUFFIX);
    write_atomic(&file, &RequestFile::new(&placed).to_string())?;
    Ok(relative(root, &file))
}

/// Creates many requests in `folder`, after its siblings and in the given order, reading the
/// folder once. `finish` may add to each file's text (an import's `[openapi]` link). Returns
/// each request's path and the text written.
pub fn create_requests(
    root: &Path,
    folder: &str,
    requests: &[StoredRequest],
    finish: impl Fn(usize, String) -> String,
) -> Result<Vec<(String, String)>, FormatError> {
    let dir = resolve(root, folder, true)?;
    let mut order = last_order(root, &dir)?;
    let mut written = Vec::with_capacity(requests.len());
    for (index, stored) in requests.iter().enumerate() {
        order += ORDER_STEP;
        let placed = StoredRequest {
            order,
            ..stored.clone()
        };
        let file = unique(&dir, &stored.name, REQUEST_SUFFIX);
        let text = finish(index, RequestFile::new(&placed).to_string());
        write_atomic(&file, &text)?;
        written.push((relative(root, &file), text));
    }
    Ok(written)
}

pub fn create_folder(root: &Path, parent: &str, name: &str) -> Result<String, FormatError> {
    let dir = resolve(root, parent, true)?;
    let order = last_order(root, &dir)? + ORDER_STEP;
    let folder = unique(&dir, name, "");
    fs::create_dir(&folder)?;
    let info = FolderInfo {
        name: name.to_owned(),
        order,
    };
    write_atomic(&folder.join(FOLDER_FILE), &files::write_folder("", &info)?)?;
    Ok(relative(root, &folder))
}

fn rewrite(
    root: &Path,
    path: &str,
    change: impl FnOnce(StoredRequest) -> StoredRequest,
) -> Result<(), FormatError> {
    let read = read_request(root, path)?;
    save_request(root, path, &change(read.stored), &read.hash).map(|_| ())
}

fn slug_file(name: &str) -> String {
    format!("{}{REQUEST_SUFFIX}", super::paths::slug(name))
}

/// Renames the request and its file; returns the new path (the same one when the name gives
/// the same file name).
pub fn rename_request(root: &Path, path: &str, name: &str) -> Result<String, FormatError> {
    let full = resolve(root, path, false)?;
    rewrite(root, path, |s| StoredRequest {
        name: name.to_owned(),
        ..s
    })?;
    let dir = full.parent().unwrap_or(root);
    if full.file_name() == Some(slug_file(name).as_ref()) {
        return Ok(path.to_owned());
    }
    let target = unique(dir, name, REQUEST_SUFFIX);
    fs::rename(&full, &target)?;
    Ok(relative(root, &target))
}

/// A copy named "<name> copy", right after the original.
pub fn duplicate_request(root: &Path, path: &str) -> Result<String, FormatError> {
    let full = resolve(root, path, false)?;
    let read = read_request(root, path)?;
    let name = format!("{} copy", read.stored.name);
    let copy = StoredRequest {
        name: name.clone(),
        order: read.stored.order + 1,
        ..read.stored
    };
    let dir = full.parent().unwrap_or(root);
    let file = unique(dir, &name, REQUEST_SUFFIX);
    write_atomic(&file, &RequestFile::new(&copy).to_string())?;
    Ok(relative(root, &file))
}

/// Moves a request, or a folder with everything in it, into `folder` (`""`: the collection),
/// last among its new siblings; returns the new path. A folder cannot move into itself or one
/// of its own folders (task 11.1).
pub fn move_node(root: &Path, path: &str, folder: &str) -> Result<String, FormatError> {
    let full = resolve(root, path, false)?;
    let dir = resolve(root, folder, true)?;
    if full.parent() == Some(dir.as_path()) {
        return Ok(path.to_owned());
    }
    if full.is_dir() {
        move_folder(root, &full, &dir)
    } else {
        move_request_file(root, path, &full, &dir)
    }
}

fn file_name_of(full: &Path) -> String {
    full.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn move_request_file(
    root: &Path,
    path: &str,
    full: &Path,
    dir: &Path,
) -> Result<String, FormatError> {
    let order = last_order(root, dir)? + ORDER_STEP;
    rewrite(root, path, |s| StoredRequest { order, ..s })?;
    let name = file_name_of(full);
    let stem = name.strip_suffix(REQUEST_SUFFIX).unwrap_or(&name);
    let target = unique(dir, stem, REQUEST_SUFFIX);
    fs::rename(full, &target)?;
    Ok(relative(root, &target))
}

fn move_folder(root: &Path, full: &Path, dir: &Path) -> Result<String, FormatError> {
    if dir.starts_with(full) {
        return Err(FormatError::Path(
            "A folder cannot move into itself or one of its folders".into(),
        ));
    }
    let order = last_order(root, dir)? + ORDER_STEP;
    let target = unique(dir, &file_name_of(full), "");
    fs::rename(full, &target)?;
    let moved = relative(root, &target);
    set_order(root, &moved, order)?;
    Ok(moved)
}

/// Renames a folder: its name in `_folder.toml`. The directory keeps its name, so the paths of
/// what is inside (open tabs, an import's manifest) stay valid.
pub fn rename_folder(root: &Path, path: &str, name: &str) -> Result<(), FormatError> {
    let full = resolve(root, path, false)?;
    if !full.is_dir() {
        return Err(FormatError::Missing(path.to_owned()));
    }
    let marker = full.join(FOLDER_FILE);
    let text = fs::read_to_string(&marker).unwrap_or_default();
    let fallback = full
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let info = FolderInfo {
        name: name.to_owned(),
        ..files::read_folder(&text, &fallback)?
    };
    write_atomic(&marker, &files::write_folder(&text, &info)?)
}

/// Puts a request or folder at `order` among its siblings.
pub fn set_order(root: &Path, path: &str, order: i64) -> Result<(), FormatError> {
    let full = resolve(root, path, false)?;
    if full.is_dir() {
        let marker = full.join(FOLDER_FILE);
        let text = fs::read_to_string(&marker).unwrap_or_default();
        let fallback = full
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let info = FolderInfo {
            order,
            ..files::read_folder(&text, &fallback)?
        };
        return write_atomic(&marker, &files::write_folder(&text, &info)?);
    }
    rewrite(root, path, |s| StoredRequest { order, ..s })
}

/// Deletes a request file, or a folder with everything in it (the UI asks first).
pub fn delete(root: &Path, path: &str) -> Result<(), FormatError> {
    let full = resolve(root, path, false)?;
    if full.is_dir() {
        fs::remove_dir_all(&full)?;
    } else {
        fs::remove_file(&full)?;
    }
    Ok(())
}
