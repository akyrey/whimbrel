//! One request per file (`*.req.toml`).

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table};
use wbl_core::{Body, Header, Request};

use crate::fields::{optional_bool, optional_integer, set, string};
use crate::{FORMAT, FormatError};

/// A request as stored: its name and place in the tree, and the request itself.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StoredRequest {
    pub name: String,
    /// Position among its siblings; gaps of 10 so a move rewrites one file.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub order: i64,
    pub request: Request,
    /// Saved examples (D-H08), in file order.
    pub examples: Vec<wbl_core::example::Example>,
    /// `[settings]`; written only when not the defaults.
    #[serde(default)]
    pub settings: wbl_core::RequestSettings,
    /// `[auth]`; inherits when absent.
    #[serde(default)]
    pub auth: wbl_core::auth::Auth,
    /// `[variables]`: request-level values, after the example and before the environment
    /// (FR-4.4); path parameters of imported requests default here.
    #[serde(default)]
    pub variables: std::collections::BTreeMap<String, String>,
    /// `[[captures]]`: values taken from the response into runtime variables (FR-6.1).
    #[serde(default)]
    pub captures: Vec<wbl_core::check::Capture>,
    /// `[[assertions]]`: checks of the response (FR-6.2).
    #[serde(default)]
    pub assertions: Vec<wbl_core::check::Assertion>,
}

/// A parsed request file. Keeps the document so writing preserves what it does not know.
#[derive(Debug, Clone)]
pub struct RequestFile {
    doc: DocumentMut,
}

impl RequestFile {
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        let doc = text
            .parse::<DocumentMut>()
            .map_err(|e| FormatError::syntax(&e))?;
        Ok(Self { doc })
    }

    /// A new file in the canonical layout.
    pub fn new(stored: &StoredRequest) -> Self {
        let mut doc = DocumentMut::new();
        set(doc.as_table_mut(), "format", FORMAT);
        Self { doc }.updated(stored)
    }

    pub fn read(&self) -> Result<StoredRequest, FormatError> {
        let root = self.doc.as_table();
        let format = optional_integer(root, "format", "format")?.unwrap_or(FORMAT);
        if format > FORMAT {
            return Err(FormatError::TooNew {
                found: format,
                supported: FORMAT,
            });
        }
        Ok(StoredRequest {
            name: string(root, "name", "name")?,
            order: optional_integer(root, "order", "order")?.unwrap_or(0),
            request: Request {
                method: string(root, "method", "method")?,
                url: string(root, "url", "url")?,
                headers: read_headers(root)?,
                body: read_body(root)?,
            },
            examples: crate::examples::read_examples(root)?,
            settings: read_settings(root)?,
            auth: crate::auth_file::read_auth(root)?,
            variables: crate::fields::string_map(root, "variables")?,
            captures: crate::checks::read_captures(root)?,
            assertions: crate::checks::read_assertions(root)?,
        })
    }

    /// This file with `stored` written into it: unchanged values, comments and unknown keys
    /// stay as they are.
    pub fn updated(&self, stored: &StoredRequest) -> Self {
        let mut doc = self.doc.clone();
        let root = doc.as_table_mut();
        set(root, "name", stored.name.as_str());
        set(root, "order", stored.order);
        set(root, "method", stored.request.method.as_str());
        set(root, "url", stored.request.url.as_str());
        write_headers(root, &stored.request.headers);
        write_body(root, &stored.request.body);
        crate::examples::write_examples(root, &stored.examples);
        write_settings(root, &stored.settings);
        crate::auth_file::write_auth(root, &stored.auth);
        if crate::fields::string_map(root, "variables").ok().as_ref() != Some(&stored.variables) {
            crate::fields::set_string_map(root, "variables", &stored.variables);
        }
        crate::checks::write_captures(root, &stored.captures);
        crate::checks::write_assertions(root, &stored.assertions);
        Self { doc }
    }
}

impl std::fmt::Display for RequestFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.doc)
    }
}

pub(crate) fn read_headers(root: &Table) -> Result<Vec<Header>, FormatError> {
    let Some(item) = root.get("headers") else {
        return Ok(Vec::new());
    };
    let tables = item
        .as_array_of_tables()
        .ok_or_else(|| FormatError::invalid("headers", "a list of [[headers]] tables"))?;
    tables
        .iter()
        .map(|t| {
            Ok(Header {
                name: string(t, "name", "headers.name")?,
                value: string(t, "value", "headers.value")?,
                enabled: optional_bool(t, "enabled", "headers.enabled", true)?,
            })
        })
        .collect()
}

pub(crate) fn write_headers(root: &mut Table, headers: &[Header]) {
    if read_headers(root).ok().as_deref() == Some(headers) {
        return;
    }
    if headers.is_empty() {
        root.remove("headers");
        return;
    }
    let mut tables = ArrayOfTables::new();
    for header in headers {
        let mut table = Table::new();
        set(&mut table, "name", header.name.as_str());
        set(&mut table, "value", header.value.as_str());
        if !header.enabled {
            set(&mut table, "enabled", false);
        }
        tables.push(table);
    }
    root.insert("headers", Item::ArrayOfTables(tables));
}

fn read_settings(root: &Table) -> Result<wbl_core::RequestSettings, FormatError> {
    let defaults = wbl_core::RequestSettings::default();
    let Some(item) = root.get("settings") else {
        return Ok(defaults);
    };
    let table = item
        .as_table()
        .ok_or_else(|| FormatError::invalid("settings", "a [settings] table"))?;
    let timeout = optional_integer(table, "timeout_ms", "settings.timeout_ms")?;
    let timeout_ms = match timeout {
        None => None,
        Some(ms) => Some(u64::try_from(ms).ok().filter(|ms| *ms > 0).ok_or_else(|| {
            FormatError::invalid("settings.timeout_ms", "a positive number of milliseconds")
        })?),
    };
    Ok(wbl_core::RequestSettings {
        timeout_ms,
        follow_redirects: optional_bool(
            table,
            "follow_redirects",
            "settings.follow_redirects",
            defaults.follow_redirects,
        )?,
        verify_tls: optional_bool(
            table,
            "verify_tls",
            "settings.verify_tls",
            defaults.verify_tls,
        )?,
    })
}

fn write_settings(root: &mut Table, settings: &wbl_core::RequestSettings) {
    if read_settings(root).ok().as_ref() == Some(settings) {
        return;
    }
    if settings.is_default() {
        root.remove("settings");
        return;
    }
    if !matches!(root.get("settings"), Some(Item::Table(_))) {
        root.insert(
            "settings",
            Item::Table(crate::examples::spaced(Table::new())),
        );
    }
    let Some(Item::Table(table)) = root.get_mut("settings") else {
        return;
    };
    match settings.timeout_ms.and_then(|ms| i64::try_from(ms).ok()) {
        Some(ms) => set(table, "timeout_ms", ms),
        None => {
            table.remove("timeout_ms");
        }
    }
    for (key, value) in [
        ("follow_redirects", settings.follow_redirects),
        ("verify_tls", settings.verify_tls),
    ] {
        if value {
            table.remove(key);
        } else {
            set(table, key, false);
        }
    }
}

fn read_body(root: &Table) -> Result<Body, FormatError> {
    match root.get("body") {
        None => Ok(Body::None),
        Some(item) => {
            let table = item
                .as_table_like()
                .ok_or_else(|| FormatError::invalid("body", "a [body] table"))?;
            crate::examples::read_body_table(table, "body", false)
        }
    }
}

fn write_body(root: &mut Table, body: &Body) {
    if read_body(root).ok().as_ref() == Some(body) {
        return;
    }
    if matches!(body, Body::None) {
        root.remove("body");
        return;
    }
    if !matches!(root.get("body"), Some(Item::Table(_))) {
        root.insert("body", Item::Table(Table::new()));
    }
    if let Some(Item::Table(table)) = root.get_mut("body") {
        crate::examples::fill_body_table(table, body);
    }
}
