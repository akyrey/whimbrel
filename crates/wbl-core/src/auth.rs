//! How a request authenticates (FR-5): set on the request, a folder or the collection, with
//! "inherit" taking the nearest one. Fields take `{{variables}}`; what they resolve to is
//! secret and masked wherever the request is shown.

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::interpolate::{Dynamic, ResolveError, Scope, interpolate};
use crate::{Header, Request};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum KeyLocation {
    Header,
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Grant {
    ClientCredentials,
    /// With PKCE, through the system browser and a loopback redirect.
    AuthorizationCode,
    /// RFC 8628: the user enters a code on another device.
    DeviceCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Config {
    pub grant: Grant,
    pub token_url: String,
    /// Authorization code grant only.
    pub auth_url: String,
    /// Device code grant only.
    pub device_url: String,
    pub client_id: String,
    /// Empty for public clients.
    pub client_secret: String,
    /// Space separated.
    pub scope: String,
}

impl OAuth2Config {
    /// Names the cached token: the same server, client, grant and scope share one.
    pub fn cache_key(&self) -> String {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0100_0000_01b3;
        let text = format!(
            "{:?}\n{}\n{}\n{}\n{}\n{}",
            self.grant, self.token_url, self.auth_url, self.device_url, self.client_id, self.scope
        );
        let value = text
            .bytes()
            .fold(OFFSET, |h, b| (h ^ u64::from(b)).wrapping_mul(PRIME));
        format!("{value:016x}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Auth {
    /// The nearest folder's, else the collection's.
    #[default]
    Inherit,
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
    ApiKey {
        name: String,
        value: String,
        location: KeyLocation,
    },
    #[serde(rename = "oauth2")]
    OAuth2(OAuth2Config),
}

/// The auth that applies: the first of `chain` (request, folders inward to outward, collection)
/// that does not inherit.
pub fn effective<'a>(chain: &[&'a Auth]) -> &'a Auth {
    chain
        .iter()
        .find(|a| !matches!(a, Auth::Inherit))
        .copied()
        .unwrap_or(&Auth::None)
}

/// An auth with its variables replaced, and the secret values it now holds.
pub type ResolvedAuthKind = Auth;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAuth {
    pub kind: ResolvedAuthKind,
    /// The credentials, in every form they are sent in, for masking.
    pub secrets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    #[error("This request needs an OAuth 2 token: get one first")]
    TokenRequired,
}

/// Replaces the variables in every field of `auth`.
pub fn resolve_auth(
    auth: &Auth,
    scopes: &[Scope],
    dynamic: &dyn Dynamic,
) -> Result<ResolvedAuth, ResolveError> {
    let mut undefined = Vec::new();
    let mut unset_secrets = Vec::new();
    let mut run = |text: &str| {
        let done = interpolate(text, scopes, dynamic);
        undefined.extend(done.undefined);
        unset_secrets.extend(done.unset_secrets);
        done.text
    };
    let kind = match auth {
        Auth::Inherit => Auth::Inherit,
        Auth::None => Auth::None,
        Auth::Basic { username, password } => Auth::Basic {
            username: run(username),
            password: run(password),
        },
        Auth::Bearer { token } => Auth::Bearer { token: run(token) },
        Auth::ApiKey {
            name,
            value,
            location,
        } => Auth::ApiKey {
            name: run(name),
            value: run(value),
            location: *location,
        },
        Auth::OAuth2(c) => Auth::OAuth2(OAuth2Config {
            token_url: run(&c.token_url),
            auth_url: run(&c.auth_url),
            device_url: run(&c.device_url),
            client_id: run(&c.client_id),
            client_secret: run(&c.client_secret),
            scope: run(&c.scope),
            grant: c.grant,
        }),
    };
    if !undefined.is_empty() || !unset_secrets.is_empty() {
        undefined.dedup();
        unset_secrets.dedup();
        return Err(ResolveError {
            undefined,
            unset_secrets,
        });
    }
    let secrets = credentials(&kind);
    Ok(ResolvedAuth { kind, secrets })
}

fn basic_value(username: &str, password: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"))
}

fn credentials(auth: &Auth) -> Vec<String> {
    let all = match auth {
        Auth::Basic { username, password } => {
            vec![password.clone(), basic_value(username, password)]
        }
        Auth::Bearer { token } => vec![token.clone()],
        Auth::ApiKey { value, .. } => vec![value.clone()],
        Auth::OAuth2(c) => vec![c.client_secret.clone()],
        Auth::Inherit | Auth::None => vec![],
    };
    all.into_iter().filter(|s| !s.is_empty()).collect()
}

fn with_header(request: &Request, name: &str, value: String) -> Request {
    let taken = request
        .enabled_headers()
        .any(|h| h.name.eq_ignore_ascii_case(name));
    if taken {
        return request.clone();
    }
    let mut headers = request.headers.clone();
    headers.push(Header {
        name: name.to_owned(),
        value,
        enabled: true,
    });
    Request {
        headers,
        ..request.clone()
    }
}

fn with_query(request: &Request, name: &str, value: &str) -> Request {
    let encode = |s: &str| url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
    let (rest, fragment) = match request.url.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment)),
        None => (request.url.as_str(), None),
    };
    let separator = if rest.contains('?') { "&" } else { "?" };
    let mut url = format!("{rest}{separator}{}={}", encode(name), encode(value));
    if let Some(fragment) = fragment {
        url.push('#');
        url.push_str(fragment);
    }
    Request {
        url,
        ..request.clone()
    }
}

/// `request` with `auth` applied; an OAuth 2 auth needs its `token`. A header the request
/// already sets is left as it is.
pub fn apply(
    request: &Request,
    auth: &ResolvedAuth,
    token: Option<&str>,
) -> Result<Request, AuthError> {
    Ok(match &auth.kind {
        Auth::Inherit | Auth::None => request.clone(),
        Auth::Basic { username, password } => with_header(
            request,
            "Authorization",
            format!("Basic {}", basic_value(username, password)),
        ),
        Auth::Bearer { token } => with_header(request, "Authorization", format!("Bearer {token}")),
        Auth::ApiKey {
            name,
            value,
            location: KeyLocation::Header,
        } => with_header(request, name, value.clone()),
        Auth::ApiKey {
            name,
            value,
            location: KeyLocation::Query,
        } => with_query(request, name, value),
        Auth::OAuth2(_) => {
            let token = token.ok_or(AuthError::TokenRequired)?;
            with_header(request, "Authorization", format!("Bearer {token}"))
        }
    })
}

#[cfg(test)]
mod tests;
