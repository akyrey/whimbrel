//! The auth a request is sent with (FR-5): its own, else its folders', else its collection's,
//! with variables resolved like the request's, and every credential masked (D-H12).

use std::path::Path;

use serde_json::json;
use wbl_core::auth::{Auth, AuthError, ResolvedAuth, apply, effective, resolve_auth};
use wbl_core::interpolate::{Resolved, Scope, SystemDynamic};
use wbl_format::collection::{self, auth};

use crate::args::RequestSendArgs;
use crate::error::EngineError;

/// Tokens of requests outside any collection are cached under this id.
pub const NO_COLLECTION: &str = "no-collection";

pub struct AuthContext {
    pub auth: ResolvedAuth,
    /// Keys the cached OAuth 2 token.
    pub collection_id: String,
}

fn chain(root: Option<&Path>, args: &RequestSendArgs) -> Result<Vec<Auth>, EngineError> {
    let mut chain = vec![args.auth.clone().unwrap_or_default()];
    if let Some(root) = root {
        match &args.path {
            Some(path) => chain.extend(auth::auth_chain(root, path)?),
            None => chain.push(auth::auth_of(root, "")?),
        }
    }
    Ok(chain)
}

pub fn context(
    root: Option<&Path>,
    args: &RequestSendArgs,
    scopes: &[Scope],
) -> Result<AuthContext, EngineError> {
    let links = chain(root, args)?;
    let refs: Vec<&Auth> = links.iter().collect();
    let auth = resolve_auth(effective(&refs), scopes, &SystemDynamic).map_err(|e| {
        EngineError::new("unresolved", e.to_string())
            .with_detail(json!({ "undefined": e.undefined, "unsetSecrets": e.unset_secrets }))
    })?;
    let collection_id = match root {
        Some(root) => collection::id(root)?,
        None => NO_COLLECTION.to_owned(),
    };
    Ok(AuthContext {
        auth,
        collection_id,
    })
}

/// The request with its auth applied; credentials and the token join the masked secrets.
pub fn finish(
    resolved: Resolved,
    context: &AuthContext,
    token: Option<&wbl_auth::Token>,
) -> Result<Resolved, EngineError> {
    let request = apply(
        &resolved.request,
        &context.auth,
        token.map(|t| t.access_token.as_str()),
    )
    .map_err(|e| match e {
        AuthError::TokenRequired => EngineError::new("oauth_required", e.to_string()),
    })?;
    let mut secrets = resolved.secrets;
    secrets.extend(context.auth.secrets.iter().cloned());
    secrets.extend(token.map(|t| t.access_token.clone()));
    Ok(Resolved { request, secrets })
}
