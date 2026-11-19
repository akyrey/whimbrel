//! Re-import (FR-7.5, D-H17): the old collection is regenerated from the kept spec and compared
//! with the new one and with the files as they are, field by field.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use wbl_format::collection::{self, slug};
use wbl_format::{FormatError, RequestFile, StoredRequest};

use crate::generate::{GeneratedRequest, generate};
use crate::install::{Source, extension, linked, manifest};
use crate::model::Api;
use crate::stored::{Entry, entries, specs};

/// The parts of a request compared on re-import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Name,
    Method,
    Url,
    Headers,
    Body,
    Auth,
    Variables,
    Examples,
}

const FIELDS: [Field; 8] = [
    Field::Name,
    Field::Method,
    Field::Url,
    Field::Headers,
    Field::Body,
    Field::Auth,
    Field::Variables,
    Field::Examples,
];

/// Which side wins a conflicting field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    /// The user's edit (the default).
    Mine,
    /// The new spec's.
    Theirs,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Conflict {
    pub field: Field,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub old: Value,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub new: Value,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub current: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum Change {
    Added {
        operation_id: String,
        name: String,
    },
    /// Left the spec; the request stays, marked.
    Removed {
        operation_id: String,
        file: String,
    },
    /// Not edited since the import: regenerated.
    Updated {
        operation_id: String,
        file: String,
    },
    /// Edited: the spec's changes merged in; `conflicts` changed on both sides.
    Merged {
        operation_id: String,
        file: String,
        conflicts: Vec<Conflict>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Plan {
    pub changes: Vec<Change>,
    /// Operations whose request was deleted by hand: left alone.
    pub skipped: Vec<String>,
}

fn field_value(stored: &StoredRequest, field: Field) -> Value {
    let r = &stored.request;
    let value = match field {
        Field::Name => serde_json::to_value(&stored.name),
        Field::Method => serde_json::to_value(&r.method),
        Field::Url => serde_json::to_value(&r.url),
        Field::Headers => serde_json::to_value(&r.headers),
        Field::Body => serde_json::to_value(&r.body),
        Field::Auth => serde_json::to_value(&stored.auth),
        Field::Variables => serde_json::to_value(&stored.variables),
        Field::Examples => serde_json::to_value(&stored.examples),
    };
    value.unwrap_or(Value::Null)
}

/// `into` with `field` taken from `from`.
fn with_field(into: StoredRequest, from: &StoredRequest, field: Field) -> StoredRequest {
    let request = match field {
        Field::Method => wbl_core::Request {
            method: from.request.method.clone(),
            ..into.request
        },
        Field::Url => wbl_core::Request {
            url: from.request.url.clone(),
            ..into.request
        },
        Field::Headers => wbl_core::Request {
            headers: from.request.headers.clone(),
            ..into.request
        },
        Field::Body => wbl_core::Request {
            body: from.request.body.clone(),
            ..into.request
        },
        _ => into.request,
    };
    let base = StoredRequest { request, ..into };
    match field {
        Field::Name => StoredRequest {
            name: from.name.clone(),
            ..base
        },
        Field::Auth => StoredRequest {
            auth: from.auth.clone(),
            ..base
        },
        Field::Variables => StoredRequest {
            variables: from.variables.clone(),
            ..base
        },
        Field::Examples => StoredRequest {
            examples: from.examples.clone(),
            ..base
        },
        _ => base,
    }
}

/// The three-way merge of one request; conflicts resolved by `choices` (mine by default).
fn merge(
    old: &StoredRequest,
    new: &StoredRequest,
    current: &StoredRequest,
    choices: &BTreeMap<Field, Choice>,
) -> (StoredRequest, Vec<Conflict>, bool) {
    let mut merged = current.clone();
    let mut conflicts = Vec::new();
    let mut changed = false;
    for field in FIELDS {
        let (o, n, c) = (
            field_value(old, field),
            field_value(new, field),
            field_value(current, field),
        );
        if n == o || n == c {
            continue;
        }
        changed = true;
        if c == o {
            merged = with_field(merged, new, field);
            continue;
        }
        conflicts.push(Conflict {
            field,
            old: o,
            new: n,
            current: c,
        });
        if choices.get(&field) == Some(&Choice::Theirs) {
            merged = with_field(merged, new, field);
        }
    }
    (merged, conflicts, changed)
}

fn read_current(root: &Path, file: &str) -> Result<Option<(String, StoredRequest)>, FormatError> {
    let path = root.join(file);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    let stored = RequestFile::parse(&text)?.read()?;
    Ok(Some((text, stored)))
}

struct Sides {
    old: BTreeMap<String, GeneratedRequest>,
    new: Vec<GeneratedRequest>,
    entries: BTreeMap<String, Entry>,
}

fn sides(root: &Path, dir: &str, old: &Api, new: &Api) -> Result<Sides, FormatError> {
    Ok(Sides {
        old: generate(old)
            .requests
            .into_iter()
            .map(|r| (r.operation_id.clone(), r))
            .collect(),
        new: generate(new).requests,
        entries: entries(root, dir)?,
    })
}

/// What re-importing `new` over the spec kept in `dir` (imported from `old`) would do.
pub fn plan(root: &Path, dir: &str, old: &Api, new: &Api) -> Result<Plan, FormatError> {
    let s = sides(root, dir, old, new)?;
    let mut out = Plan::default();
    let none = BTreeMap::new();
    for request in &s.new {
        let id = &request.operation_id;
        let Some(entry) = s.entries.get(id) else {
            out.changes.push(Change::Added {
                operation_id: id.clone(),
                name: request.stored.name.clone(),
            });
            continue;
        };
        let Some((text, current)) = read_current(root, &entry.file)? else {
            out.skipped.push(id.clone());
            continue;
        };
        let old_stored = s
            .old
            .get(id)
            .map_or_else(|| request.stored.clone(), |o| o.stored.clone());
        if collection::hash(text.as_bytes()) == entry.hash {
            if old_stored != request.stored {
                out.changes.push(Change::Updated {
                    operation_id: id.clone(),
                    file: entry.file.clone(),
                });
            }
            continue;
        }
        let (_, conflicts, changed) = merge(&old_stored, &request.stored, &current, &none);
        if changed {
            out.changes.push(Change::Merged {
                operation_id: id.clone(),
                file: entry.file.clone(),
                conflicts,
            });
        }
    }
    let ids: BTreeSet<&String> = s.new.iter().map(|r| &r.operation_id).collect();
    for (id, entry) in &s.entries {
        if !ids.contains(id) && !entry.removed && root.join(&entry.file).is_file() {
            out.changes.push(Change::Removed {
                operation_id: id.clone(),
                file: entry.file.clone(),
            });
        }
    }
    Ok(out)
}

/// The text of `file` with `stored` written in, keeping its order, comments and `[openapi]`.
fn rewrite(text: &str, stored: &StoredRequest) -> Result<String, FormatError> {
    let file = RequestFile::parse(text)?;
    let order = file.read()?.order;
    Ok(file
        .updated(&StoredRequest {
            order,
            ..stored.clone()
        })
        .to_string())
}

fn mark_removed(text: &str) -> Result<String, FormatError> {
    let mut doc: toml_edit::DocumentMut = text.parse().map_err(|e| FormatError::syntax(&e))?;
    doc["openapi"]["removed"] = toml_edit::value(true);
    Ok(doc.to_string())
}

/// Folders (by name chain under `base`) as they are on disk, for added requests.
fn folders_for(
    root: &Path,
    base: &str,
    added: &[&GeneratedRequest],
) -> Result<BTreeMap<Vec<String>, String>, FormatError> {
    let needed: Vec<Vec<String>> = added
        .iter()
        .flat_map(|r| (1..=r.folder.len()).map(|d| r.folder[..d].to_vec()))
        .collect();
    let mut existing = BTreeMap::new();
    for chain in &needed {
        let mut dir = base.to_owned();
        for name in chain {
            dir = if dir.is_empty() {
                slug(name)
            } else {
                format!("{dir}/{}", slug(name))
            };
        }
        if root.join(&dir).is_dir() {
            existing.insert(chain.clone(), dir);
        }
    }
    let missing: Vec<Vec<String>> = needed
        .into_iter()
        .filter(|c| !existing.contains_key(c))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let generated = crate::generate::Generated {
        name: String::new(),
        auth: wbl_core::auth::Auth::Inherit,
        folders: missing,
        requests: Vec::new(),
        environments: Vec::new(),
        warnings: Vec::new(),
    };
    // Parents found on disk are used for new children.
    let mut created = existing.clone();
    for (chain, dir) in create_folders_under(root, base, &generated, &existing)? {
        created.insert(chain, dir);
    }
    Ok(created)
}

/// Creates the folders of `generated` that are not in `existing` (unlike the installer, which
/// creates them all).
fn create_folders_under(
    root: &Path,
    base: &str,
    generated: &crate::generate::Generated,
    existing: &BTreeMap<Vec<String>, String>,
) -> Result<BTreeMap<Vec<String>, String>, FormatError> {
    let mut dirs = existing.clone();
    for chain in &generated.folders {
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
    Ok(dirs)
}

/// Applies the re-import: `choices` settles conflicts per operation and field (mine by default).
pub fn apply(
    root: &Path,
    dir: &str,
    old: &Api,
    new: &Api,
    source: &Source,
    choices: &BTreeMap<String, BTreeMap<Field, Choice>>,
) -> Result<Plan, FormatError> {
    let done = plan(root, dir, old, new)?;
    let s = sides(root, dir, old, new)?;
    let kept = specs(root)?
        .into_iter()
        .find(|x| x.dir == dir)
        .ok_or_else(|| FormatError::Missing(format!("{dir}/import.toml")))?;
    let mut files: BTreeMap<String, (String, String)> = s
        .entries
        .iter()
        .map(|(id, e)| (id.clone(), (e.file.clone(), e.hash.clone())))
        .collect();
    let mut removed: BTreeSet<String> = s
        .entries
        .iter()
        .filter(|(_, e)| e.removed)
        .map(|(id, _)| id.clone())
        .collect();
    let by_id: BTreeMap<&String, &GeneratedRequest> =
        s.new.iter().map(|r| (&r.operation_id, r)).collect();
    let added: Vec<&GeneratedRequest> = done
        .changes
        .iter()
        .filter_map(|c| match c {
            Change::Added { operation_id, .. } => by_id.get(operation_id).copied(),
            _ => None,
        })
        .collect();
    let dirs = folders_for(root, &kept.folder, &added)?;
    for request in &added {
        let folder = dirs
            .get(&request.folder)
            .cloned()
            .unwrap_or_else(|| kept.folder.clone());
        let id = request.operation_id.clone();
        let written = collection::create_requests(
            root,
            &folder,
            std::slice::from_ref(&request.stored),
            |_, text| linked(&text, &id),
        )?;
        for (path, text) in written {
            files.insert(id.clone(), (path, collection::hash(text.as_bytes())));
        }
    }
    let none = BTreeMap::new();
    for change in &done.changes {
        match change {
            Change::Added { .. } => {}
            Change::Removed { operation_id, file } => {
                let text = mark_removed(&fs::read_to_string(root.join(file))?)?;
                collection::write_atomic(&root.join(file), &text)?;
                removed.insert(operation_id.clone());
            }
            Change::Updated { operation_id, file }
            | Change::Merged {
                operation_id, file, ..
            } => {
                let Some(request) = by_id.get(operation_id) else {
                    continue;
                };
                let Some((text, current)) = read_current(root, file)? else {
                    continue;
                };
                let old_stored = s
                    .old
                    .get(operation_id)
                    .map_or_else(|| request.stored.clone(), |o| o.stored.clone());
                let picked = choices.get(operation_id).unwrap_or(&none);
                let (merged, _, _) = merge(&old_stored, &request.stored, &current, picked);
                let written = rewrite(&text, &merged)?;
                collection::write_atomic(&root.join(file), &written)?;
                // An untouched file stays "untouched": its hash is what was written. An edited
                // one keeps the generated text's hash, so the next re-import merges again.
                let generated = rewrite(&text, &request.stored)?;
                let hash = if matches!(change, Change::Updated { .. }) {
                    &written
                } else {
                    &generated
                };
                files.insert(
                    operation_id.clone(),
                    (file.clone(), collection::hash(hash.as_bytes())),
                );
            }
        }
    }
    let spec_dir = root.join(dir);
    for ext in ["json", "yaml"] {
        let old_source = spec_dir.join(format!("source.{ext}"));
        if old_source.is_file() {
            fs::remove_file(old_source)?;
        }
    }
    collection::write_atomic(
        &spec_dir.join(format!("source.{}", extension(&source.text))),
        &source.text,
    )?;
    let title = if new.title.is_empty() {
        kept.title.clone()
    } else {
        new.title.clone()
    };
    collection::write_atomic(
        &spec_dir.join(crate::install::MANIFEST),
        &manifest(&title, source, &kept.folder, &files, &removed),
    )?;
    Ok(done)
}
