//! One send for callers outside the app (the MCP server, D-H23): no registry and no history,
//! and nothing it returns holds a secret value.

use tokio_util::sync::CancellationToken;
use wbl_core::Request;
use wbl_core::interpolate::{Value, mask, mask_request};
use wbl_http::{Response, Sender};

use crate::args::RequestSendArgs;
use crate::checks::{CaptureResult, Checks, after_send};
use crate::error::EngineError;
use crate::mask::masked_response;
use crate::resolve::labeled_scopes;
use crate::runtime::runtime_key;
use crate::send::{Sources, ready};

/// The clients a send goes through: one connection pool, one OAuth 2 client.
#[derive(Clone, Copy)]
pub struct Clients<'a> {
    pub sender: &'a Sender,
    pub oauth: &'a wbl_auth::OAuthClient,
}

/// A send as it may leave the process: the request and response with every known secret
/// masked, the body cut to 256 KB, and the request's checks.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Delivered {
    pub request: Request,
    pub response: Response,
    pub checks: Checks,
}

/// Every secret value the request could have seen or the response could echo: the ones it
/// used (credentials, the OAuth 2 token) and every secret in scope after the send, including
/// what a secret capture just took.
fn known_secrets(
    sources: &Sources<'_>,
    args: &RequestSendArgs,
    used: &[String],
) -> Result<Vec<String>, EngineError> {
    let runtime = sources.runtime.scope(&runtime_key(sources.root))?;
    let scopes = labeled_scopes(
        sources.root,
        args.environment.as_deref(),
        None,
        None,
        runtime,
        sources.secrets,
    )?;
    let in_scope = scopes.iter().flat_map(|(_, scope)| {
        scope.entries().filter_map(|(_, value)| match value {
            Value::Secret(Some(v)) => Some(v.clone()),
            _ => None,
        })
    });
    Ok(used.iter().cloned().chain(in_scope).collect())
}

fn masked_checks(checks: Checks, secrets: &[String]) -> Checks {
    Checks {
        assertions: checks
            .assertions
            .into_iter()
            .map(|a| wbl_core::check::CheckResult {
                message: mask(&a.message, secrets),
                ..a
            })
            .collect(),
        captures: checks
            .captures
            .into_iter()
            .map(|c| CaptureResult {
                value: c.value.map(|v| mask(&v, secrets)),
                error: c.error.map(|e| mask(&e, secrets)),
                ..c
            })
            .collect(),
    }
}

/// Sends `args` once with its checks. Errors carry no secret value either.
pub async fn send_once(
    clients: Clients<'_>,
    sources: &Sources<'_>,
    args: &RequestSendArgs,
    cancel: &CancellationToken,
) -> Result<Delivered, EngineError> {
    let ready = ready(clients.oauth, sources, args).await?;
    let used = &ready.resolved.secrets;
    let sent = clients
        .sender
        .send_with(
            &ready.resolved.request,
            &ready.exchange,
            cancel,
            &mut |_| {},
        )
        .await;
    let response: Response = match sent {
        Ok(response) => response,
        Err(error) => {
            let error = EngineError::from(error);
            return Err(EngineError {
                message: mask(&error.message, used),
                ..error
            });
        }
    };
    let checks = after_send(
        sources.runtime,
        sources.secrets,
        sources.root,
        args,
        &response,
    )?;
    let known = known_secrets(sources, args, used)?;
    Ok(Delivered {
        request: mask_request(&ready.resolved.request, &known),
        response: masked_response(&response, &known),
        checks: masked_checks(checks, &known),
    })
}

#[cfg(test)]
mod tests;
