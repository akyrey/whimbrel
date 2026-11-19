//! Example values for parameters and bodies, and how they are written on the wire.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use url::form_urlencoded;

use wbl_core::{Body, BodyKind, FormField};

use crate::model::{MediaType, Parameter};
use crate::synth::{Direction, example};

/// Text of a scalar; arrays joined with commas; objects as JSON.
pub fn text_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Array(items) => items.iter().map(text_of).collect::<Vec<_>>().join(","),
        other => other.to_string(),
    }
}

fn encode(text: &str) -> String {
    form_urlencoded::byte_serialize(text.as_bytes()).collect()
}

/// A parameter's value: its example, its first named example, else one from its schema.
pub fn param_value(param: &Parameter, schemas: &BTreeMap<String, Value>) -> Value {
    param
        .example
        .clone()
        .or_else(|| param.examples.iter().find_map(|e| e.value.clone()))
        .or_else(|| {
            param
                .schema
                .as_ref()
                .map(|s| example(s, schemas, Direction::Request))
        })
        .unwrap_or(Value::Null)
}

/// `name=value` pairs of a query parameter, by its `style` and `explode` (OAS 3 §4.8.12.4).
pub fn query_pairs(param: &Parameter, value: &Value) -> Vec<(String, String)> {
    let style = param.style.as_deref().unwrap_or("form");
    let explode = param.explode.unwrap_or(style == "form");
    let name = &param.name;
    match value {
        Value::Array(items) if explode => {
            items.iter().map(|v| (name.clone(), text_of(v))).collect()
        }
        Value::Array(items) => {
            let separator = match style {
                "spaceDelimited" => " ",
                "pipeDelimited" => "|",
                _ => ",",
            };
            vec![(
                name.clone(),
                items
                    .iter()
                    .map(text_of)
                    .collect::<Vec<_>>()
                    .join(separator),
            )]
        }
        Value::Object(map) if explode => map.iter().map(|(k, v)| (k.clone(), text_of(v))).collect(),
        Value::Object(map) => {
            vec![(
                name.clone(),
                map.iter()
                    .flat_map(|(k, v)| [k.clone(), text_of(v)])
                    .collect::<Vec<_>>()
                    .join(","),
            )]
        }
        other => vec![(name.clone(), text_of(other))],
    }
}

pub fn query_string(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// A form body (`application/x-www-form-urlencoded`) of an object value.
pub fn form(value: &Value) -> String {
    match value {
        Value::Object(map) => query_string(
            &map.iter()
                .map(|(k, v)| (k.clone(), text_of(v)))
                .collect::<Vec<_>>(),
        ),
        other => text_of(other),
    }
}

/// How a body of this media type is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    Json,
    Form,
    Xml,
    Text,
    Multipart,
    /// Anything else: a file sent as it is.
    Binary,
}

pub fn wire(media_type: &str) -> Wire {
    let essence = media_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match essence.as_str() {
        m if m == "application/json" || m.ends_with("+json") || m == "*/*" => Wire::Json,
        "application/x-www-form-urlencoded" => Wire::Form,
        m if m.starts_with("multipart/") => Wire::Multipart,
        m if m.ends_with("/xml") || m.ends_with("+xml") => Wire::Xml,
        m if m.starts_with("text/") || m == "application/jsonl" || m == "application/x-ndjson" => {
            Wire::Text
        }
        _ => Wire::Binary,
    }
}

/// The media type a request is generated with: JSON first, then forms, text, XML.
pub fn preferred(content: &[MediaType]) -> Option<&MediaType> {
    let rank = |m: &MediaType| match wire(&m.media_type) {
        Wire::Json => 0,
        Wire::Form => 1,
        Wire::Multipart => 2,
        Wire::Text => 3,
        Wire::Xml => 4,
        Wire::Binary => 5,
    };
    content.iter().min_by_key(|m| rank(m))
}

/// A text body's text for `value`.
fn body_text(wire: Wire, value: &Value) -> String {
    match (wire, value) {
        (Wire::Json, v) => serde_json::to_string_pretty(v).unwrap_or_default(),
        (_, Value::String(s)) => s.clone(),
        (Wire::Xml, _) => String::new(),
        (_, v) => text_of(v),
    }
}

fn kind_of(wire: Wire) -> BodyKind {
    match wire {
        Wire::Json => BodyKind::Json,
        Wire::Xml => BodyKind::Xml,
        _ => BodyKind::Text,
    }
}

/// Form fields of an object value (`files`: the ones sent as files, left without a path).
fn fields_of(value: Option<&Value>, files: &BTreeSet<String>) -> Vec<FormField> {
    let Some(Value::Object(map)) = value else {
        return Vec::new();
    };
    map.iter()
        .map(|(name, v)| {
            let file = files.contains(name);
            FormField {
                name: name.clone(),
                value: if file { String::new() } else { text_of(v) },
                enabled: true,
                file,
            }
        })
        .collect()
}

/// Fields from a serialized form (`a=1&b=2`).
fn fields_from_query(text: &str) -> Vec<FormField> {
    form_urlencoded::parse(text.as_bytes())
        .map(|(name, value)| FormField {
            name: name.into_owned(),
            value: value.into_owned(),
            enabled: true,
            file: false,
        })
        .collect()
}

/// A body of `wire` for `value` (or its serialized form); `files` names multipart file parts.
pub fn body_for(
    wire: Wire,
    value: Option<&Value>,
    serialized: Option<&str>,
    files: &BTreeSet<String>,
) -> Body {
    match wire {
        Wire::Form => Body::Form {
            fields: serialized.map_or_else(|| fields_of(value, files), fields_from_query),
        },
        Wire::Multipart => Body::Multipart {
            fields: fields_of(value, files),
        },
        Wire::Binary => Body::File {
            path: String::new(),
        },
        text => Body::Text {
            kind: kind_of(text),
            text: serialized
                .map(str::to_owned)
                .or_else(|| value.map(|v| body_text(text, v)))
                .unwrap_or_default(),
        },
    }
}

/// Properties of an object schema that are files (`format: binary`), following one `$ref`.
pub fn file_properties(
    schema: Option<&Value>,
    schemas: &BTreeMap<String, Value>,
) -> BTreeSet<String> {
    let resolved = schema.and_then(|s| {
        s.get("$ref")
            .and_then(Value::as_str)
            .and_then(|r| r.strip_prefix("#/schemas/"))
            .and_then(|name| schemas.get(name))
            .or(Some(s))
    });
    resolved
        .and_then(|s| s.get("properties"))
        .and_then(Value::as_object)
        .map(|props| {
            props
                .iter()
                .filter(|(_, p)| {
                    let format = p.get("format").and_then(Value::as_str);
                    let items = p
                        .get("items")
                        .and_then(|i| i.get("format"))
                        .and_then(Value::as_str);
                    matches!(format, Some("binary" | "base64")) || items == Some("binary")
                })
                .map(|(k, _)| k.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The body's default value: the first named example, the media type's example, else one from
/// the schema.
pub fn body_value(media: &MediaType, schemas: &BTreeMap<String, Value>) -> Option<Value> {
    media
        .examples
        .iter()
        .find_map(|e| e.value.clone())
        .or_else(|| media.example.clone())
        .or_else(|| {
            media
                .schema
                .as_ref()
                .map(|s| example(s, schemas, Direction::Request))
        })
}
