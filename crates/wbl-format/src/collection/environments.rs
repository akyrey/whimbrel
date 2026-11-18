//! A collection's environments: `environments/<id>.toml`, with `.local/<id>.toml` beside for the
//! user's overrides. An environment's id is its file name; it keys the keychain entries.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::paths::{hash, slug, unique, write_atomic};
use crate::environment::{EnvironmentFile, read_local, write_local};
use crate::{Environment, FormatError};

const DIR: &str = "environments";
const LOCAL_DIR: &str = ".local";
const SUFFIX: &str = ".toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnvironmentEntry {
    pub id: String,
    /// `None` when the file cannot be read; `error` says why.
    pub name: Option<String>,
    pub protected: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnvironmentRead {
    pub id: String,
    pub environment: Environment,
    pub local: BTreeMap<String, String>,
    pub hash: String,
}

/// The file of environment `id`; only ids that are slugs (what `create` makes) are accepted.
fn file(root: &Path, dir: &str, id: &str) -> Result<PathBuf, FormatError> {
    if id.is_empty() || slug(id) != id {
        return Err(FormatError::Path(format!(
            "{id:?} is not an environment of this collection"
        )));
    }
    Ok(root.join(dir).join(format!("{id}{SUFFIX}")))
}

pub fn list(root: &Path) -> Result<Vec<EnvironmentEntry>, FormatError> {
    let dir = root.join(DIR);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(id) = name.strip_suffix(SUFFIX) else {
            continue;
        };
        if id.starts_with('.') {
            continue;
        }
        let read = fs::read_to_string(&path)
            .map_err(FormatError::from)
            .and_then(|t| EnvironmentFile::parse(&t))
            .and_then(|f| f.read());
        entries.push(match read {
            Ok(env) => EnvironmentEntry {
                id: id.to_owned(),
                name: Some(env.name),
                protected: env.protected,
                error: None,
            },
            Err(e) => EnvironmentEntry {
                id: id.to_owned(),
                name: None,
                protected: false,
                error: Some(e.to_string()),
            },
        });
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

pub fn read(root: &Path, id: &str) -> Result<EnvironmentRead, FormatError> {
    let text = fs::read_to_string(file(root, DIR, id)?)?;
    let environment = EnvironmentFile::parse(&text)?.read()?;
    let local_path = file(root, LOCAL_DIR, id)?;
    let local = match fs::read_to_string(&local_path) {
        Ok(local) => read_local(&local)?,
        Err(_) => BTreeMap::new(),
    };
    Ok(EnvironmentRead {
        id: id.to_owned(),
        environment,
        local,
        hash: hash(text.as_bytes()),
    })
}

/// Creates the environment; returns its id (a unique slug of its name).
pub fn create(root: &Path, environment: &Environment) -> Result<String, FormatError> {
    let dir = root.join(DIR);
    fs::create_dir_all(&dir)?;
    let path = unique(&dir, &environment.name, SUFFIX);
    write_atomic(&path, &EnvironmentFile::new(environment).to_string())?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(name.strip_suffix(SUFFIX).unwrap_or(&name).to_owned())
}

/// Saves the environment if its file is still what was read; returns the new hash.
pub fn save(
    root: &Path,
    id: &str,
    environment: &Environment,
    expected_hash: &str,
) -> Result<String, FormatError> {
    let path = file(root, DIR, id)?;
    let current = fs::read_to_string(&path)?;
    if hash(current.as_bytes()) != expected_hash {
        return Err(FormatError::Conflict);
    }
    let text = EnvironmentFile::parse(&current)?
        .updated(environment)
        .to_string();
    if text != current {
        write_atomic(&path, &text)?;
    }
    Ok(hash(text.as_bytes()))
}

pub fn delete(root: &Path, id: &str) -> Result<(), FormatError> {
    fs::remove_file(file(root, DIR, id)?)?;
    let local = file(root, LOCAL_DIR, id)?;
    if local.exists() {
        fs::remove_file(local)?;
    }
    Ok(())
}

/// Writes the user's overrides, and makes sure git ignores them.
pub fn save_local(
    root: &Path,
    id: &str,
    variables: &BTreeMap<String, String>,
) -> Result<(), FormatError> {
    let path = file(root, LOCAL_DIR, id)?;
    fs::create_dir_all(root.join(LOCAL_DIR))?;
    let current = fs::read_to_string(&path).unwrap_or_default();
    write_atomic(&path, &write_local(&current, variables)?)?;
    ensure_ignored(root)
}

fn ensure_ignored(root: &Path) -> Result<(), FormatError> {
    let ignore = root.join(".gitignore");
    let text = fs::read_to_string(&ignore).unwrap_or_default();
    if text
        .lines()
        .any(|l| l.trim() == ".local/" || l.trim() == ".local")
    {
        return Ok(());
    }
    let separator = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    write_atomic(&ignore, &format!("{text}{separator}.local/\n"))
}
