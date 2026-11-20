//! Auth on folders (`_folder.toml`) and the collection (`collection.toml`), the chain a request
//! inherits through, and the collection's TLS.

use std::fs;
use std::path::{Path, PathBuf};

use wbl_core::auth::Auth;

use super::files::parse_doc;
use super::paths::{COLLECTION_FILE, FOLDER_FILE, resolve, write_atomic};
use crate::auth_file::{CollectionTls, read_auth, read_tls, write_auth, write_tls};
use crate::{FORMAT, FormatError};

/// The file holding what `folder` (`""`: the collection) passes to its requests.
fn holder(root: &Path, folder: &str) -> Result<PathBuf, FormatError> {
    if folder.is_empty() {
        return Ok(root.join(COLLECTION_FILE));
    }
    let dir = resolve(root, folder, false)?;
    if !dir.is_dir() {
        return Err(FormatError::Path(format!(
            "{folder:?} is not a folder of this collection"
        )));
    }
    Ok(dir.join(FOLDER_FILE))
}

fn read_text(path: &Path) -> Result<String, FormatError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.into()),
    }
}

/// The `_folder.toml` or `collection.toml` of `folder`, parsed.
pub(super) fn read_holder(
    root: &Path,
    folder: &str,
) -> Result<toml_edit::DocumentMut, FormatError> {
    parse_doc(&read_text(&holder(root, folder)?)?)
}

/// Rewrites the holder of `folder` with `edit` applied; a new `_folder.toml` gets a format.
pub(super) fn edit_holder(
    root: &Path,
    folder: &str,
    edit: impl FnOnce(&mut toml_edit::Table),
) -> Result<(), FormatError> {
    let path = holder(root, folder)?;
    let mut doc = parse_doc(&read_text(&path)?)?;
    if !folder.is_empty() && !doc.contains_key("format") {
        crate::fields::set(doc.as_table_mut(), "format", FORMAT);
    }
    edit(doc.as_table_mut());
    write_atomic(&path, &doc.to_string())
}

/// The folders around `request`, innermost first (the collection not included).
pub(super) fn folders_around(request: &str) -> Vec<&str> {
    let mut folders: Vec<&str> = Vec::new();
    let mut path = request;
    while let Some((parent, _)) = path.rsplit_once('/') {
        folders.push(parent);
        path = parent;
    }
    folders
}

pub fn auth_of(root: &Path, folder: &str) -> Result<Auth, FormatError> {
    read_auth(read_holder(root, folder)?.as_table())
}

pub fn save_auth(root: &Path, folder: &str, auth: &Auth) -> Result<(), FormatError> {
    edit_holder(root, folder, |t| write_auth(t, auth))
}

/// The auth of each folder around `request` (innermost first), then the collection's: what
/// "inherit" looks through. A folder whose `_folder.toml` cannot be read counts as inheriting.
pub fn auth_chain(root: &Path, request: &str) -> Result<Vec<Auth>, FormatError> {
    let mut chain: Vec<Auth> = folders_around(request)
        .iter()
        .map(|f| auth_of(root, f).unwrap_or(Auth::Inherit))
        .collect();
    chain.push(auth_of(root, "")?);
    Ok(chain)
}

pub fn tls(root: &Path) -> Result<CollectionTls, FormatError> {
    read_tls(parse_doc(&read_text(&root.join(COLLECTION_FILE))?)?.as_table())
}

pub fn save_tls(root: &Path, settings: &CollectionTls) -> Result<(), FormatError> {
    let path = root.join(COLLECTION_FILE);
    let mut doc = parse_doc(&read_text(&path)?)?;
    write_tls(doc.as_table_mut(), settings);
    write_atomic(&path, &doc.to_string())
}

/// A file named by the collection's TLS settings, inside the collection or absolute.
pub fn tls_file(root: &Path, name: &str) -> PathBuf {
    let path = Path::new(name);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
