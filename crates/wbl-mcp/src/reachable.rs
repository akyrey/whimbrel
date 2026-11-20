//! The collections a server may touch: those named with `--collection` when it started
//! (FR-10.5). Every tool names one, by its folder or its name; nothing else is reachable.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::McpError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reachable {
    pub root: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct Collections(Vec<Reachable>);

impl Collections {
    /// Checks each folder holds a collection; the first that does not is the error.
    pub fn open(folders: &[PathBuf]) -> Result<Self, McpError> {
        folders
            .iter()
            .map(|folder| {
                let root = folder
                    .canonicalize()
                    .map_err(|e| McpError::NotReachable(format!("{}: {e}", folder.display())))?;
                let name = wbl_format::collection::load(&root)
                    .map_err(|e| {
                        McpError::NotReachable(format!(
                            "{} is not a Whimbrel collection: {e}",
                            root.display()
                        ))
                    })?
                    .name;
                Ok(Reachable { root, name })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }

    pub fn list(&self) -> &[Reachable] {
        &self.0
    }

    /// The root of `collection`: one of the folders exactly, or else the one collection with
    /// that name (ignoring case).
    pub fn root(&self, collection: &str) -> Result<&Path, McpError> {
        let by_path = Path::new(collection)
            .canonicalize()
            .ok()
            .and_then(|p| self.0.iter().find(|c| c.root == p));
        if let Some(found) = by_path {
            return Ok(&found.root);
        }
        let named: Vec<&Reachable> = self
            .0
            .iter()
            .filter(|c| c.name.eq_ignore_ascii_case(collection))
            .collect();
        match named.as_slice() {
            [one] => Ok(&one.root),
            [] => Err(McpError::NotReachable(format!(
                "{collection} is not one of this server's collections; list_collections names them"
            ))),
            _ => Err(McpError::InvalidArgument(format!(
                "Several collections are named {collection}; name one by its folder"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn collection(name: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        wbl_format::collection::create(dir.path(), name).unwrap();
        dir
    }

    #[test]
    fn a_collection_is_found_by_folder_or_name() {
        let api = collection("API");
        let other = collection("Other");
        let open = Collections::open(&[api.path().into(), other.path().into()]).unwrap();
        let root = api.path().canonicalize().unwrap();

        assert_eq!(open.root(&api.path().to_string_lossy()).unwrap(), root);
        assert_eq!(open.root("api").unwrap(), root);
        assert_eq!(open.list().len(), 2);
        assert_eq!(open.root("nope").unwrap_err().code(), "not_reachable");
    }

    #[test]
    fn a_folder_elsewhere_is_not_reachable() {
        let api = collection("API");
        let elsewhere = collection("Elsewhere");
        let open = Collections::open(&[api.path().into()]).unwrap();

        let error = open.root(&elsewhere.path().to_string_lossy()).unwrap_err();

        assert_eq!(error.code(), "not_reachable");
    }

    #[test]
    fn two_collections_with_one_name_need_a_folder() {
        let one = collection("API");
        let two = collection("API");
        let open = Collections::open(&[one.path().into(), two.path().into()]).unwrap();

        assert_eq!(open.root("API").unwrap_err().code(), "invalid_argument");
    }

    #[test]
    fn a_folder_without_a_collection_is_refused_at_start() {
        let empty = tempfile::tempdir().unwrap();
        let error = Collections::open(&[empty.path().into()]).unwrap_err();
        assert_eq!(error.code(), "not_reachable");
    }
}
