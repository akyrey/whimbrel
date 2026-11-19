//! Turning a request as written into the request that is sent: the example laid over it, then
//! its variables resolved from the example, `.local`, the environment (with its secrets from
//! the keychain) and the collection.

use std::path::Path;

use serde_json::json;
use wbl_core::Request;
use wbl_core::example::{Example, apply};
use wbl_core::interpolate::{Resolved, Scope, SystemDynamic, Value, resolve};
use wbl_format::collection::{self, environments};

use crate::error::AppError;
use crate::secrets::Secrets;

/// Secrets this short are not looked for in saved files: they would match ordinary text.
const MIN_GUARDED_SECRET: usize = 4;

fn plain(map: &std::collections::BTreeMap<String, String>) -> Scope {
    Scope::new(
        map.iter()
            .map(|(k, v)| (k.clone(), Value::Plain(v.clone())))
            .collect(),
    )
}

/// Where a scope's values come from, for the editor's hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Example,
    Request,
    Runtime,
    Local,
    Environment,
    Collection,
    Dynamic,
}

/// The scopes for `environment` of the collection at `root`, highest precedence first, each
/// with where it comes from.
pub fn labeled_scopes(
    root: Option<&Path>,
    environment: Option<&str>,
    example: Option<&Example>,
    request_variables: Option<&std::collections::BTreeMap<String, String>>,
    runtime: Scope,
    secrets: &Secrets,
) -> Result<Vec<(Source, Scope)>, AppError> {
    let mut scopes: Vec<(Source, Scope)> = example
        .map(|e| (Source::Example, e.scope()))
        .into_iter()
        .collect();
    scopes.extend(request_variables.map(|v| (Source::Request, plain(v))));
    scopes.push((Source::Runtime, runtime));
    let Some(root) = root else { return Ok(scopes) };
    if let Some(id) = environment {
        let read = environments::read(root, id)?;
        let collection_id = collection::id(root)?;
        scopes.push((Source::Local, plain(&read.local)));
        let mut values: std::collections::BTreeMap<String, Value> = read
            .environment
            .variables
            .iter()
            .map(|(k, v)| (k.clone(), Value::Plain(v.clone())))
            .collect();
        for secret in &read.environment.secrets {
            let value = secrets.get(&collection_id, id, &secret.name)?;
            values.insert(secret.name.clone(), Value::Secret(value));
        }
        scopes.push((Source::Environment, Scope::new(values)));
    }
    scopes.push((Source::Collection, plain(&collection::variables(root)?)));
    Ok(scopes)
}

/// The scopes alone, highest precedence first.
pub fn scopes(
    root: Option<&Path>,
    environment: Option<&str>,
    example: Option<&Example>,
    request_variables: Option<&std::collections::BTreeMap<String, String>>,
    runtime: Scope,
    secrets: &Secrets,
) -> Result<Vec<Scope>, AppError> {
    Ok(labeled_scopes(
        root,
        environment,
        example,
        request_variables,
        runtime,
        secrets,
    )?
    .into_iter()
    .map(|(_, scope)| scope)
    .collect())
}

/// A variable the editor can complete and explain.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
pub struct ScopeVariable {
    pub name: String,
    /// `None` for a secret (never sent to the frontend) and for dynamic values.
    pub value: Option<String>,
    pub source: Source,
    pub secret: bool,
    /// A secret with a value in the keychain.
    pub set: bool,
}

/// Every variable in scope, each once (the one that wins), then the dynamic ones.
pub fn in_scope(scopes: &[(Source, Scope)]) -> Vec<ScopeVariable> {
    let mut out: Vec<ScopeVariable> = Vec::new();
    for (source, scope) in scopes {
        for (name, value) in scope.entries() {
            if out.iter().any(|v| v.name == *name) {
                continue;
            }
            out.push(match value {
                Value::Plain(v) => ScopeVariable {
                    name: name.clone(),
                    value: Some(v.clone()),
                    source: *source,
                    secret: false,
                    set: true,
                },
                Value::Secret(v) => ScopeVariable {
                    name: name.clone(),
                    value: None,
                    source: *source,
                    secret: true,
                    set: v.is_some(),
                },
            });
        }
    }
    out.extend(
        wbl_core::interpolate::DYNAMIC_NAMES
            .iter()
            .map(|n| ScopeVariable {
                name: format!("${n}"),
                value: None,
                source: Source::Dynamic,
                secret: false,
                set: true,
            }),
    );
    out
}

/// The request that is sent, or `unresolved` naming what is missing.
pub fn resolved(
    request: &Request,
    example: Option<&Example>,
    scopes: &[Scope],
) -> Result<Resolved, AppError> {
    let base = example.map_or_else(|| request.clone(), |e| apply(request, e));
    resolve(&base, scopes, &SystemDynamic).map_err(|e| {
        AppError::new("unresolved", e.to_string())
            .with_detail(json!({ "undefined": e.undefined, "unsetSecrets": e.unset_secrets }))
    })
}

/// `request` with every secret value replaced by the mask, for showing.
pub fn masked(resolved: &Resolved) -> Request {
    wbl_core::interpolate::mask_request(&resolved.request, &resolved.secrets)
}

/// Refuses to write `text` into the collection when it holds the value of one of the
/// collection's secrets (FR-2.8.5); the error names the secret, never its value.
pub fn guard(root: &Path, text: &str, secrets: &Secrets) -> Result<(), AppError> {
    let collection_id = collection::id(root)?;
    for entry in environments::list(root)? {
        if entry.error.is_some() {
            continue;
        }
        let read = environments::read(root, &entry.id)?;
        for secret in &read.environment.secrets {
            let Some(value) = secrets.get(&collection_id, &entry.id, &secret.name)? else {
                continue;
            };
            if value.chars().count() >= MIN_GUARDED_SECRET && text.contains(&value) {
                return Err(AppError::new(
                    "contains_secret",
                    format!(
                        "This holds the value of the secret {} ({}). Use {{{{{}}}}} instead.",
                        secret.name, read.environment.name, secret.name
                    ),
                ));
            }
        }
    }
    Ok(())
}
