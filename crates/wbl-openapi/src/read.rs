//! Text to JSON values: JSON as it is, YAML (1.2) converted, and loading every document a spec
//! refers to before the model is built.

use std::collections::BTreeMap;
use std::future::Future;

use serde_json::{Map, Number, Value};
use url::Url;
use yaml_rust2::{Yaml, YamlLoader};

use crate::OpenApiError;

/// Reads JSON or YAML (JSON is YAML, but the JSON parser is faster and stricter).
pub fn parse(text: &str) -> Result<Value, OpenApiError> {
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('{') {
        return serde_json::from_str(trimmed)
            .map_err(|e| OpenApiError::Syntax(format!("JSON: {e}")));
    }
    let docs = YamlLoader::load_from_str(trimmed)
        .map_err(|e| OpenApiError::Syntax(format!("YAML: {e}")))?;
    let first = docs
        .into_iter()
        .next()
        .ok_or_else(|| OpenApiError::Syntax("The document is empty".into()))?;
    Ok(to_json(first))
}

fn to_json(yaml: Yaml) -> Value {
    match yaml {
        Yaml::Real(text) => text
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map_or(Value::String(text), Value::Number),
        Yaml::Integer(n) => Value::Number(n.into()),
        Yaml::String(s) => Value::String(s),
        Yaml::Boolean(b) => Value::Bool(b),
        Yaml::Array(items) => Value::Array(items.into_iter().map(to_json).collect()),
        Yaml::Hash(hash) => {
            let mut map = Map::new();
            for (key, value) in hash {
                let key = match key {
                    Yaml::String(s) => s,
                    // Status codes (200), versions (1.0) and booleans are keys too.
                    Yaml::Integer(n) => n.to_string(),
                    Yaml::Real(r) => r,
                    Yaml::Boolean(b) => b.to_string(),
                    other => format!("{other:?}"),
                };
                map.insert(key, to_json(value));
            }
            Value::Object(map)
        }
        Yaml::Null | Yaml::BadValue | Yaml::Alias(_) => Value::Null,
    }
}

/// Fetches a referenced document by its URL (`file:` or `http(s):`).
pub trait Loader {
    fn load(&self, url: &Url) -> impl Future<Output = Result<String, OpenApiError>> + Send;
}

/// A spec and every document it refers to, by URL without fragment.
#[derive(Debug, Clone, Default)]
pub struct Documents {
    pub entry: Option<Url>,
    pub by_url: BTreeMap<String, Value>,
}

fn base_of(url: &Url) -> String {
    let mut base = url.clone();
    base.set_fragment(None);
    base.to_string()
}

impl Documents {
    pub fn entry_value(&self) -> Option<&Value> {
        self.entry
            .as_ref()
            .and_then(|u| self.by_url.get(&base_of(u)))
    }

    pub fn get(&self, url: &Url) -> Option<&Value> {
        self.by_url.get(&base_of(url))
    }
}

fn external_refs(value: &Value, base: &Url, out: &mut Vec<Url>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(target)) = map.get("$ref")
                && !target.starts_with('#')
                && let Ok(url) = base.join(target)
            {
                out.push(url);
            }
            map.values().for_each(|v| external_refs(v, base, out));
        }
        Value::Array(items) => items.iter().for_each(|v| external_refs(v, base, out)),
        _ => {}
    }
}

/// A spec with what it refers to, and what could not be read.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    pub documents: Documents,
    pub warnings: Vec<String>,
    /// Remote documents referred to but not fetched (remote references were off).
    pub remote: Vec<String>,
}

/// The entry document (`text`, found at `url`) and everything it refers to. Remote documents
/// are fetched only with `allow_remote`; otherwise they are listed in [`Loaded::remote`].
pub async fn load_all(
    url: Url,
    text: &str,
    loader: &impl Loader,
    allow_remote: bool,
) -> Result<Loaded, OpenApiError> {
    let mut loaded = Loaded {
        documents: Documents {
            entry: Some(url.clone()),
            by_url: BTreeMap::new(),
        },
        ..Loaded::default()
    };
    let mut queue = vec![(url, parse(text)?)];
    while let Some((at, value)) = queue.pop() {
        let mut found = Vec::new();
        external_refs(&value, &at, &mut found);
        loaded.documents.by_url.insert(base_of(&at), value);
        for target in found {
            let key = base_of(&target);
            let seen = loaded.documents.by_url.contains_key(&key)
                || loaded.remote.contains(&key)
                || queue.iter().any(|(u, _)| base_of(u) == key);
            if seen {
                continue;
            }
            if matches!(target.scheme(), "http" | "https") && !allow_remote {
                loaded.remote.push(key);
                continue;
            }
            match loader.load(&target).await {
                Ok(text) => queue.push((target, parse(&text)?)),
                Err(e) => loaded.warnings.push(format!("Could not read {key}: {e}")),
            }
        }
    }
    Ok(loaded)
}
