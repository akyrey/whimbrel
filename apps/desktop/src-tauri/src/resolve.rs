//! Turning a request as written into the request that is sent: the example laid over it, then
//! its variables resolved from the example, `.local`, the environment (with its secrets from
//! the keychain) and the collection.

use std::path::Path;

use serde_json::json;
use wbl_core::Request;
use wbl_core::example::{Example, apply};
use wbl_core::interpolate::{Resolved, Scope, SystemDynamic, Value, mask, resolve};
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

/// The scopes for `environment` of the collection at `root`, highest precedence first.
pub fn scopes(
    root: Option<&Path>,
    environment: Option<&str>,
    example: Option<&Example>,
    request_variables: Option<&std::collections::BTreeMap<String, String>>,
    secrets: &Secrets,
) -> Result<Vec<Scope>, AppError> {
    let mut scopes: Vec<Scope> = example.map(Example::scope).into_iter().collect();
    scopes.extend(request_variables.map(plain));
    let Some(root) = root else { return Ok(scopes) };
    if let Some(id) = environment {
        let read = environments::read(root, id)?;
        let collection_id = collection::id(root)?;
        scopes.push(plain(&read.local));
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
        scopes.push(Scope::new(values));
    }
    scopes.push(plain(&collection::variables(root)?));
    Ok(scopes)
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
    let m = |text: &str| mask(text, &resolved.secrets);
    let r = &resolved.request;
    Request {
        method: r.method.clone(),
        url: m(&r.url),
        headers: r
            .headers
            .iter()
            .map(|h| wbl_core::Header {
                name: m(&h.name),
                value: m(&h.value),
                enabled: h.enabled,
            })
            .collect(),
        body: match &r.body {
            wbl_core::Body::None => wbl_core::Body::None,
            wbl_core::Body::Text { kind, text } => wbl_core::Body::Text {
                kind: *kind,
                text: m(text),
            },
        },
    }
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
