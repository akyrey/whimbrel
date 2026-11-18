//! Paths inside a collection: what the frontend may name, file names from titles, atomic writes.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::FormatError;

pub const REQUEST_SUFFIX: &str = ".req.toml";
pub const COLLECTION_FILE: &str = "collection.toml";
pub const FOLDER_FILE: &str = "_folder.toml";
/// Folders with a meaning of their own, not request folders (FILE_FORMAT.md).
pub const RESERVED_DIRS: [&str; 2] = ["environments", "openapi"];

/// `relative` resolved inside `root`, or an error: no absolute paths, no `..`, nothing hidden
/// (`.git`, `.local`), never the collection file itself. The empty path is the root.
pub fn resolve(root: &Path, relative: &str, allow_root: bool) -> Result<PathBuf, FormatError> {
    let refuse = || FormatError::Path(format!("{relative:?} is not a path in this collection"));
    if relative.is_empty() {
        return if allow_root {
            Ok(root.to_path_buf())
        } else {
            Err(refuse())
        };
    }
    let path = Path::new(relative);
    let mut resolved = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) if !part.to_string_lossy().starts_with('.') => {
                resolved.push(part);
            }
            _ => return Err(refuse()),
        }
    }
    if path == Path::new(COLLECTION_FILE) {
        return Err(refuse());
    }
    Ok(resolved)
}

/// The path of `full` relative to `root`, with `/` separators (what the frontend sees).
pub fn relative(root: &Path, full: &Path) -> String {
    full.strip_prefix(root)
        .unwrap_or(full)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Lower-case ASCII letters and digits joined by single dashes; `request` when nothing is left.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() {
        "request".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `dir/<slug><suffix>`, with `-2`, `-3`… when taken.
pub fn unique(dir: &Path, name: &str, suffix: &str) -> PathBuf {
    let base = slug(name);
    let mut candidate = dir.join(format!("{base}{suffix}"));
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{base}-{n}{suffix}"));
        n += 1;
    }
    candidate
}

/// Writes through a temporary file in the same folder, then renames: a crash leaves either the
/// old file or the new one, never half of one.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), FormatError> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = path.with_file_name(format!(".{name}.tmp"));
    fs::write(&temp, text)?;
    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        FormatError::from(e)
    })
}

/// FNV-1a: stable across runs and versions, enough to tell whether a file changed.
pub fn hash(bytes: &[u8]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let value = bytes
        .iter()
        .fold(OFFSET, |h, b| (h ^ u64::from(*b)).wrapping_mul(PRIME));
    format!("{value:016x}")
}
