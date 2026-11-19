//! Writes an [`Imported`] collection under a folder of a collection.

use std::collections::BTreeMap;
use std::path::Path;

use wbl_format::FormatError;
use wbl_format::collection::{self, environments};

use crate::Imported;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename = "InteropInstalled"))]
pub struct Installed {
    /// Request files written, relative to the collection.
    pub requests: Vec<String>,
    /// Ids of the environments created.
    pub environments: Vec<String>,
    pub warnings: Vec<String>,
}

/// Writes `imported` under `base` (`""`: the collection's top). Collection variables are added
/// to the collection's own (existing names keep their value); environments whose name is taken
/// are not created again.
pub fn install(root: &Path, base: &str, imported: &Imported) -> Result<Installed, FormatError> {
    let mut dirs: BTreeMap<Vec<String>, String> = BTreeMap::new();
    for chain in &imported.folders {
        let Some((name, parents)) = chain.split_last() else {
            continue;
        };
        let parent = dirs
            .get(parents)
            .cloned()
            .unwrap_or_else(|| base.to_owned());
        dirs.insert(
            chain.clone(),
            collection::create_folder(root, &parent, name)?,
        );
    }
    collection::auth::save_auth(root, base, &imported.auth)?;
    let mut by_folder: BTreeMap<String, Vec<wbl_format::StoredRequest>> = BTreeMap::new();
    for r in &imported.requests {
        let dir = dirs
            .get(&r.folder)
            .cloned()
            .unwrap_or_else(|| base.to_owned());
        by_folder.entry(dir).or_default().push(r.stored.clone());
    }
    let mut installed = Installed {
        warnings: imported.warnings.clone(),
        ..Installed::default()
    };
    for (dir, stored) in by_folder {
        let written = collection::create_requests(root, &dir, &stored, |_, text| text)?;
        installed
            .requests
            .extend(written.into_iter().map(|(path, _)| path));
    }
    if !imported.variables.is_empty() {
        let mut variables = collection::variables(root)?;
        for (k, v) in &imported.variables {
            variables.entry(k.clone()).or_insert_with(|| v.clone());
        }
        collection::save_variables(root, &variables)?;
    }
    let existing: Vec<String> = environments::list(root)?
        .into_iter()
        .filter_map(|e| e.name)
        .collect();
    for environment in &imported.environments {
        if existing.contains(&environment.name) {
            installed.warnings.push(format!(
                "The environment {} exists; it was left as it is",
                environment.name
            ));
            continue;
        }
        installed
            .environments
            .push(environments::create(root, environment)?);
    }
    Ok(installed)
}
