//! `$ref`: following references between and inside documents, and the schema table that keeps
//! referenced schemas once, as links, so cycles are never expanded.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use url::Url;

use crate::read::Documents;

/// How many `$ref`s in a row are followed before giving up (a reference loop).
const MAX_CHAIN: usize = 32;

pub struct Resolver<'a> {
    docs: &'a Documents,
    /// Absolute reference → its name in the schema table.
    names: BTreeMap<String, String>,
    pub schemas: BTreeMap<String, Value>,
    pending: Vec<(Url, String)>,
    pub warnings: Vec<String>,
    /// 3.0 schemas use `nullable` and a single `example`.
    pub legacy_schemas: bool,
}

fn unescape(token: &str) -> String {
    let decoded = url::form_urlencoded::parse(format!("x={token}").as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| token.to_owned());
    decoded.replace("~1", "/").replace("~0", "~")
}

/// The value at a JSON pointer fragment (`/components/schemas/Pet`).
fn pointer<'v>(value: &'v Value, fragment: &str) -> Option<&'v Value> {
    fragment.split('/').skip(1).try_fold(value, |at, token| {
        let key = unescape(token);
        match at {
            Value::Object(map) => map.get(&key),
            Value::Array(items) => key.parse::<usize>().ok().and_then(|i| items.get(i)),
            _ => None,
        }
    })
}

impl<'a> Resolver<'a> {
    pub fn new(docs: &'a Documents) -> Self {
        Self {
            docs,
            names: BTreeMap::new(),
            schemas: BTreeMap::new(),
            pending: Vec::new(),
            warnings: Vec::new(),
            legacy_schemas: false,
        }
    }

    /// The value a reference names, and the URL further references inside it are relative to.
    pub fn lookup(&self, base: &Url, reference: &str) -> Option<(Url, &'a Value)> {
        let url = base.join(reference).ok()?;
        let doc = self.docs.get(&url)?;
        let target = match url.fragment() {
            Some(fragment) if !fragment.is_empty() => pointer(doc, fragment)?,
            _ => doc,
        };
        Some((url, target))
    }

    /// `value` with `$ref`s followed (not for schemas, which stay links). A broken reference is
    /// a warning and an empty object.
    pub fn deref(&mut self, base: &Url, value: &'a Value) -> (Url, &'a Value) {
        let mut at = (base.clone(), value);
        for _ in 0..MAX_CHAIN {
            let Some(Value::String(reference)) = at.1.get("$ref") else {
                return at;
            };
            match self.lookup(&at.0, reference) {
                Some(next) => at = next,
                None => {
                    self.warnings
                        .push(format!("A reference leads nowhere: {reference}"));
                    return (at.0, &EMPTY);
                }
            }
        }
        self.warnings
            .push("A chain of references does not end".into());
        (at.0, &EMPTY)
    }

    /// Gives a component schema its name up front, so names follow the document.
    pub fn register(&mut self, url: &Url) {
        self.name_for(url);
    }

    fn name_for(&mut self, url: &Url) -> String {
        let key = url.to_string();
        if let Some(name) = self.names.get(&key) {
            return name.clone();
        }
        let fragment = url.fragment().unwrap_or("");
        let last = fragment.rsplit('/').find(|s| !s.is_empty()).map(unescape);
        let stem = url
            .path_segments()
            .and_then(|mut s| s.next_back())
            .map(|f| f.split('.').next().unwrap_or(f).to_owned());
        let base = last
            .or(stem)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Schema".into());
        let mut name = base.clone();
        let mut n = 2;
        while self.names.values().any(|v| *v == name) {
            name = format!("{base}{n}");
            n += 1;
        }
        self.names.insert(key, name.clone());
        self.pending.push((url.clone(), name.clone()));
        name
    }

    /// A schema with its references made links into the table (`#/schemas/<name>`), and 3.0's
    /// `nullable` / `example` written as 3.1 does.
    pub fn schema(&mut self, base: &Url, value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(reference)) = map.get("$ref") {
                    return match base.join(reference) {
                        Ok(url) => {
                            let name = self.name_for(&url);
                            let mut link = Map::new();
                            link.insert("$ref".into(), Value::String(format!("#/schemas/{name}")));
                            // 3.1 allows keywords next to $ref (a description, say).
                            for (k, v) in map.iter().filter(|(k, _)| *k != "$ref") {
                                link.insert(k.clone(), self.schema(base, v));
                            }
                            Value::Object(link)
                        }
                        Err(_) => value.clone(),
                    };
                }
                let mut out: Map<String, Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), self.schema(base, v)))
                    .collect();
                if self.legacy_schemas {
                    legacy(&mut out);
                }
                Value::Object(out)
            }
            Value::Array(items) => {
                Value::Array(items.iter().map(|v| self.schema(base, v)).collect())
            }
            other => other.clone(),
        }
    }

    /// Reads every schema the links point to, including those found on the way.
    pub fn finish(&mut self) {
        while let Some((url, name)) = self.pending.pop() {
            let base = url.clone();
            let value = match self.lookup(&base, url.as_str()) {
                Some((at, target)) => self.schema(&at, target),
                None => {
                    self.warnings
                        .push(format!("A schema reference leads nowhere: {url}"));
                    Value::Object(Map::new())
                }
            };
            self.schemas.insert(name, value);
        }
    }
}

static EMPTY: Value = Value::Null;

/// 3.0 → 3.1: `nullable: true` joins `null` to the type; `example` becomes `examples: [..]`.
fn legacy(schema: &mut Map<String, Value>) {
    if schema.remove("nullable") == Some(Value::Bool(true)) {
        let types = match schema.remove("type") {
            Some(Value::String(t)) => vec![Value::String(t), Value::String("null".into())],
            Some(Value::Array(mut ts)) => {
                ts.push(Value::String("null".into()));
                ts
            }
            _ => vec![Value::String("null".into())],
        };
        schema.insert("type".into(), Value::Array(types));
    }
    if let Some(example) = schema.remove("example") {
        schema
            .entry("examples")
            .or_insert_with(|| Value::Array(vec![example]));
    }
}
