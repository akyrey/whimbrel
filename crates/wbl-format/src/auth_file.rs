//! `[auth]` (request, `_folder.toml`, `collection.toml`) and `[tls]` (`collection.toml`).
//! An absent `[auth]` inherits.

use toml_edit::{Item, Table};
use wbl_core::auth::{Auth, Grant, KeyLocation, OAuth2Config};

use crate::FormatError;
use crate::examples::spaced;
use crate::fields::{set, string};

fn optional_string(table: &Table, key: &str, field: &str) -> Result<String, FormatError> {
    match table.get(key) {
        None => Ok(String::new()),
        Some(_) => string(table, key, field),
    }
}

fn grant_of(name: &str) -> Option<Grant> {
    match name {
        "client_credentials" => Some(Grant::ClientCredentials),
        "authorization_code" => Some(Grant::AuthorizationCode),
        "device_code" => Some(Grant::DeviceCode),
        _ => None,
    }
}

fn grant_name(grant: Grant) -> &'static str {
    match grant {
        Grant::ClientCredentials => "client_credentials",
        Grant::AuthorizationCode => "authorization_code",
        Grant::DeviceCode => "device_code",
    }
}

const TYPES: &str = "one of none, basic, bearer, api_key, oauth2";

fn read_oauth(t: &Table) -> Result<Auth, FormatError> {
    let grant = string(t, "grant", "auth.grant")?;
    let grant = grant_of(&grant).ok_or_else(|| {
        FormatError::invalid(
            "auth.grant",
            "one of client_credentials, authorization_code, device_code",
        )
    })?;
    Ok(Auth::OAuth2(OAuth2Config {
        grant,
        token_url: string(t, "token_url", "auth.token_url")?,
        auth_url: optional_string(t, "auth_url", "auth.auth_url")?,
        device_url: optional_string(t, "device_url", "auth.device_url")?,
        client_id: string(t, "client_id", "auth.client_id")?,
        client_secret: optional_string(t, "client_secret", "auth.client_secret")?,
        scope: optional_string(t, "scope", "auth.scope")?,
        redirect_uri: optional_string(t, "redirect_uri", "auth.redirect_uri")?,
    }))
}

pub(crate) fn read_auth(root: &Table) -> Result<Auth, FormatError> {
    let Some(item) = root.get("auth") else {
        return Ok(Auth::Inherit);
    };
    let t = item
        .as_table()
        .ok_or_else(|| FormatError::invalid("auth", "an [auth] table"))?;
    match string(t, "type", "auth.type")?.as_str() {
        "inherit" => Ok(Auth::Inherit),
        "none" => Ok(Auth::None),
        "basic" => Ok(Auth::Basic {
            username: string(t, "username", "auth.username")?,
            password: optional_string(t, "password", "auth.password")?,
        }),
        "bearer" => Ok(Auth::Bearer {
            token: string(t, "token", "auth.token")?,
        }),
        "api_key" => {
            let location = match optional_string(t, "location", "auth.location")?.as_str() {
                "" | "header" => KeyLocation::Header,
                "query" => KeyLocation::Query,
                _ => return Err(FormatError::invalid("auth.location", "header or query")),
            };
            Ok(Auth::ApiKey {
                name: string(t, "name", "auth.name")?,
                value: string(t, "value", "auth.value")?,
                location,
            })
        }
        "oauth2" => read_oauth(t),
        _ => Err(FormatError::invalid("auth.type", TYPES)),
    }
}

fn fill(t: &mut Table, auth: &Auth) {
    let mut put = |key: &str, value: &str, always: bool| {
        if always || !value.is_empty() {
            set(t, key, value);
        }
    };
    match auth {
        Auth::Inherit => put("type", "inherit", true),
        Auth::None => put("type", "none", true),
        Auth::Basic { username, password } => {
            put("type", "basic", true);
            put("username", username, true);
            put("password", password, false);
        }
        Auth::Bearer { token } => {
            put("type", "bearer", true);
            put("token", token, true);
        }
        Auth::ApiKey {
            name,
            value,
            location,
        } => {
            put("type", "api_key", true);
            put("name", name, true);
            put("value", value, true);
            put(
                "location",
                if *location == KeyLocation::Query {
                    "query"
                } else {
                    "header"
                },
                true,
            );
        }
        Auth::OAuth2(c) => {
            put("type", "oauth2", true);
            put("grant", grant_name(c.grant), true);
            put("token_url", &c.token_url, true);
            put("auth_url", &c.auth_url, false);
            put("device_url", &c.device_url, false);
            put("client_id", &c.client_id, true);
            put("client_secret", &c.client_secret, false);
            put("scope", &c.scope, false);
            put("redirect_uri", &c.redirect_uri, false);
        }
    }
}

/// Writes `auth` only when it changed; "inherit" removes the table.
pub(crate) fn write_auth(root: &mut Table, auth: &Auth) {
    if read_auth(root).ok().as_ref() == Some(auth) {
        return;
    }
    if *auth == Auth::Inherit {
        root.remove("auth");
        return;
    }
    let mut t = spaced(Table::new());
    fill(&mut t, auth);
    root.insert("auth", Item::Table(t));
}

/// A collection's own TLS: extra trusted CA and a client certificate, both PEM files given
/// relative to the collection (they may be git-ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CollectionTls {
    pub ca_file: String,
    /// Certificate and private key in one PEM file.
    pub client_cert_file: String,
}

pub(crate) fn read_tls(root: &Table) -> Result<CollectionTls, FormatError> {
    let Some(item) = root.get("tls") else {
        return Ok(CollectionTls::default());
    };
    let t = item
        .as_table()
        .ok_or_else(|| FormatError::invalid("tls", "a [tls] table"))?;
    Ok(CollectionTls {
        ca_file: optional_string(t, "ca_file", "tls.ca_file")?,
        client_cert_file: optional_string(t, "client_cert_file", "tls.client_cert_file")?,
    })
}

pub(crate) fn write_tls(root: &mut Table, tls: &CollectionTls) {
    if read_tls(root).ok().as_ref() == Some(tls) {
        return;
    }
    if *tls == CollectionTls::default() {
        root.remove("tls");
        return;
    }
    let mut t = spaced(Table::new());
    for (key, value) in [
        ("ca_file", &tls.ca_file),
        ("client_cert_file", &tls.client_cert_file),
    ] {
        if !value.is_empty() {
            set(&mut t, key, value.as_str());
        }
    }
    root.insert("tls", Item::Table(t));
}
