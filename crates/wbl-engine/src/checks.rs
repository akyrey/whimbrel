//! After a send (FR-6): the request's captures set runtime variables (secret ones saved to the
//! keychain when the environment declares them), and its assertions are evaluated.

use serde::Serialize;
use wbl_core::check::{CaptureError, CheckResult, Observed, assert, capture};
use wbl_format::collection::{self, environments};
use wbl_http::Response;

use crate::args::RequestSendArgs;
use crate::error::EngineError;
use crate::runtime::{Runtime, runtime_key};
use crate::secrets::Secrets;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CaptureResult {
    pub var: String,
    pub secret: bool,
    /// The value taken; `None` for a secret, or when nothing was taken.
    pub value: Option<String>,
    /// Why nothing was taken.
    pub error: Option<String>,
    /// Saved to the active environment's secret in the keychain.
    pub saved: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Checks {
    pub assertions: Vec<CheckResult>,
    pub captures: Vec<CaptureResult>,
}

impl Checks {
    pub fn passed(&self) -> bool {
        self.assertions.iter().all(|a| a.passed) && self.captures.iter().all(|c| c.error.is_none())
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Sent {
    pub response: Response,
    pub checks: Checks,
}

pub fn observed(response: &Response) -> Observed {
    Observed {
        status: response.status,
        headers: response
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect(),
        body: response.body.text.clone(),
        total_ms: response.timing.total_ms,
    }
}

/// Saves `value` as the environment's secret `name` when the environment declares it.
fn save_secret(
    secrets: &Secrets,
    root: &std::path::Path,
    environment: &str,
    name: &str,
    value: &str,
) -> Result<bool, EngineError> {
    let declared = environments::read(root, environment)?
        .environment
        .secrets
        .iter()
        .any(|s| s.name == name);
    if !declared {
        return Ok(false);
    }
    secrets.set(&collection::id(root)?, environment, name, value)?;
    Ok(true)
}

/// Runs the request's captures and assertions on `response`. `root` is the collection the
/// request was sent from, already checked by the caller.
pub fn after_send(
    runtime: &Runtime,
    secrets: &Secrets,
    root: Option<&std::path::Path>,
    args: &RequestSendArgs,
    response: &Response,
) -> Result<Checks, EngineError> {
    let seen = observed(response);
    let key = runtime_key(root);
    let mut captures = Vec::new();
    for c in args.captures.iter().flatten() {
        let mut result = CaptureResult {
            var: c.var.clone(),
            secret: c.secret,
            value: None,
            error: None,
            saved: false,
        };
        match capture(c, &seen) {
            Ok(value) => {
                runtime.set(&key, &c.var, value.clone(), c.secret)?;
                if c.secret {
                    if let (Some(root), Some(env)) = (root, &args.environment) {
                        result.saved = save_secret(secrets, root, env, &c.var, &value)?;
                    }
                } else {
                    result.value = Some(value);
                }
            }
            Err(e) => result.error = Some(e.to_string()),
        }
        captures.push(result);
    }
    let assertions = args
        .assertions
        .iter()
        .flatten()
        .map(|a| assert(a, &seen))
        .collect();
    Ok(Checks {
        assertions,
        captures,
    })
}

/// The values at the JSONPath `path` in `body`, as one pretty JSON array.
pub fn select(body: &str, path: &str) -> Result<String, EngineError> {
    let found = wbl_core::check::select(body, path).map_err(|e| {
        let code = match e {
            CaptureError::NotJson => "not_json",
            _ => "invalid_json_path",
        };
        EngineError::new(code, e.to_string())
    })?;
    serde_json::to_string_pretty(&found).map_err(|e| EngineError::internal(e.to_string()))
}

#[cfg(test)]
mod tests;
