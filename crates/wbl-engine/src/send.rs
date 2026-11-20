//! A request from the editor or a file to the request that goes on the wire: the headers it
//! inherits (D-H22), its example, its variables (with secrets from the keychain) and its auth.

use std::path::Path;

use wbl_core::interpolate::{Resolved, Scope};
use wbl_core::{Header, Request};
use wbl_format::collection::{self, headers};
use wbl_http::{Exchange, TlsFiles};

use crate::args::RequestSendArgs;
use crate::auth::{AuthContext, context, finish};
use crate::error::EngineError;
use crate::resolve::{ScopeVariable, in_scope, labeled_scopes, masked, resolved};
use crate::runtime::{Runtime, runtime_key};
use crate::secrets::Secrets;

/// What a send reads besides its arguments. `root` is the request's collection, already
/// checked by the caller (the app: open; the MCP server: reachable).
#[derive(Clone, Copy)]
pub struct Sources<'a> {
    pub root: Option<&'a Path>,
    pub runtime: &'a Runtime,
    pub secrets: &'a Secrets,
}

/// What resolving a request needs: its scopes, its auth and the headers it inherits.
pub struct Parts {
    pub scopes: Vec<Scope>,
    pub auth: AuthContext,
    /// Its folders' headers (innermost first), then its collection's (D-H22).
    inherited: Vec<Vec<Header>>,
}

/// The headers a request inherits: none outside a collection, the collection's when unsaved.
fn inherited_headers(
    root: Option<&Path>,
    path: Option<&str>,
) -> Result<Vec<Vec<Header>>, EngineError> {
    Ok(match (root, path) {
        (None, _) => Vec::new(),
        (Some(root), Some(path)) => headers::headers_chain(root, path)?,
        (Some(root), None) => vec![headers::headers_of(root, "")?],
    })
}

pub fn parts(sources: &Sources<'_>, args: &RequestSendArgs) -> Result<Parts, EngineError> {
    let runtime = sources.runtime.scope(&runtime_key(sources.root))?;
    let scopes = labeled_scopes(
        sources.root,
        args.environment.as_deref(),
        args.example.as_ref(),
        args.variables.as_ref(),
        runtime,
        sources.secrets,
    )?
    .into_iter()
    .map(|(_, scope)| scope)
    .collect::<Vec<_>>();
    let auth = context(sources.root, args, &scopes)?;
    let inherited = inherited_headers(sources.root, args.path.as_deref())?;
    Ok(Parts {
        scopes,
        auth,
        inherited,
    })
}

/// The request as it will be sent: inherited headers, example, variables and auth applied.
pub fn build(
    args: &RequestSendArgs,
    parts: &Parts,
    token: Option<&wbl_auth::Token>,
) -> Result<Resolved, EngineError> {
    let request = wbl_core::inherit::with_inherited(&args.request, &parts.inherited);
    let resolved = resolved(&request, args.example.as_ref(), &parts.scopes)?;
    finish(resolved, &parts.auth, token)
}

/// For showing (preview, cURL): the cached token as it is, not refreshed.
pub fn prepare(sources: &Sources<'_>, args: &RequestSendArgs) -> Result<Resolved, EngineError> {
    let parts = parts(sources, args)?;
    let token = crate::oauth::cached(sources.secrets, &parts.auth)?;
    build(args, &parts, token.as_ref())
}

/// What would be sent, secrets masked (FR-2.5).
pub fn preview(sources: &Sources<'_>, args: &RequestSendArgs) -> Result<Request, EngineError> {
    Ok(masked(&prepare(sources, args)?))
}

/// The resolved request as a cURL command (FR-2.6); secrets masked unless `reveal`.
pub fn curl(
    sources: &Sources<'_>,
    args: &RequestSendArgs,
    reveal: bool,
) -> Result<String, EngineError> {
    let resolved = prepare(sources, args)?;
    let request = if reveal {
        resolved.request.clone()
    } else {
        masked(&resolved)
    };
    Ok(wbl_core::curl::to_curl(&request))
}

/// The variables a request can use, for completion and hints (FR-2.4).
pub fn variables(
    sources: &Sources<'_>,
    args: &RequestSendArgs,
) -> Result<Vec<ScopeVariable>, EngineError> {
    let runtime = sources.runtime.scope(&runtime_key(sources.root))?;
    let labeled = labeled_scopes(
        sources.root,
        args.environment.as_deref(),
        args.example.as_ref(),
        args.variables.as_ref(),
        runtime,
        sources.secrets,
    )?;
    Ok(in_scope(&labeled))
}

/// The collection's own TLS files, read for each send (they are small, and may change).
fn tls_of(root: Option<&Path>) -> Result<TlsFiles, EngineError> {
    let Some(root) = root else {
        return Ok(TlsFiles::default());
    };
    let settings = collection::auth::tls(root)?;
    let read = |name: &str| -> Result<Option<Vec<u8>>, EngineError> {
        if name.is_empty() {
            return Ok(None);
        }
        let path = collection::auth::tls_file(root, name);
        std::fs::read(&path).map(Some).map_err(|e| {
            EngineError::new(
                "tls",
                format!("Cannot read the collection's TLS file {name}: {e}"),
            )
        })
    };
    Ok(TlsFiles {
        ca_pem: read(&settings.ca_file)?,
        client_pem: read(&settings.client_cert_file)?,
    })
}

/// A request ready to go: resolved (secret values listed for masking) and how to send it.
pub struct Ready {
    pub resolved: Resolved,
    pub exchange: Exchange,
}

/// Resolves `args` for sending: the OAuth 2 token is refreshed when stale, file bodies are read
/// from the collection, and the collection's cookie jar and TLS files are used.
pub async fn ready(
    oauth: &wbl_auth::OAuthClient,
    sources: &Sources<'_>,
    args: &RequestSendArgs,
) -> Result<Ready, EngineError> {
    let parts = parts(sources, args)?;
    let token = crate::oauth::usable(oauth, sources.secrets, &parts.auth).await?;
    let resolved = build(args, &parts, token.as_ref())?;
    let resolved = match sources.root {
        Some(root) => Resolved {
            request: Request {
                body: resolved.request.body.with_files_under(root),
                ..resolved.request
            },
            ..resolved
        },
        None => resolved,
    };
    let exchange = Exchange {
        settings: args.settings.unwrap_or_default(),
        jar: sources.root.and_then(|r| collection::id(r).ok()),
        tls: tls_of(sources.root)?,
    };
    Ok(Ready { resolved, exchange })
}

#[cfg(test)]
mod tests;
