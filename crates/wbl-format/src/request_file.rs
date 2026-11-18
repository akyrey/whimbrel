//! One request per file (`*.req.toml`).

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table};
use wbl_core::{Body, BodyKind, Header, Request};

use crate::fields::{optional_bool, optional_integer, set, string};
use crate::{FORMAT, FormatError};

/// A request as stored: its name and place in the tree, and the request itself.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StoredRequest {
    pub name: String,
    /// Position among its siblings; gaps of 10 so a move rewrites one file.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub order: i64,
    pub request: Request,
}

/// A parsed request file. Keeps the document so writing preserves what it does not know.
#[derive(Debug, Clone)]
pub struct RequestFile {
    doc: DocumentMut,
}

const BODY_KINDS: &str = "one of json, xml, text";

fn kind_name(kind: BodyKind) -> &'static str {
    match kind {
        BodyKind::Json => "json",
        BodyKind::Xml => "xml",
        BodyKind::Text => "text",
    }
}

fn kind_of(name: &str) -> Option<BodyKind> {
    match name {
        "json" => Some(BodyKind::Json),
        "xml" => Some(BodyKind::Xml),
        "text" => Some(BodyKind::Text),
        _ => None,
    }
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
            return Err(FormatError::TooNew { found: format, supported: FORMAT });
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
        Self { doc }
    }
}

impl std::fmt::Display for RequestFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.doc)
    }
}

fn read_headers(root: &Table) -> Result<Vec<Header>, FormatError> {
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

fn write_headers(root: &mut Table, headers: &[Header]) {
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

fn read_body(root: &Table) -> Result<Body, FormatError> {
    let Some(item) = root.get("body") else {
        return Ok(Body::None);
    };
    let table = item
        .as_table()
        .ok_or_else(|| FormatError::invalid("body", "a [body] table"))?;
    let kind = string(table, "kind", "body.kind")?;
    let kind = kind_of(&kind).ok_or_else(|| FormatError::invalid("body.kind", BODY_KINDS))?;
    Ok(Body::Text { kind, text: string(table, "text", "body.text")? })
}

fn write_body(root: &mut Table, body: &Body) {
    let Body::Text { kind, text } = body else {
        root.remove("body");
        return;
    };
    let table = match root.get_mut("body").and_then(Item::as_table_mut) {
        Some(table) => table,
        None => {
            root.insert("body", Item::Table(Table::new()));
            match root.get_mut("body").and_then(Item::as_table_mut) {
                Some(table) => table,
                None => return,
            }
        }
    };
    set(table, "kind", kind_name(*kind));
    set(table, "text", text.as_str());
}
