//! Example values for parameters and bodies, and how they are written on the wire.

use std::collections::BTreeMap;

use serde_json::Value;
use url::form_urlencoded;

use crate::model::{MediaType, NamedExample, Parameter};
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

/// How a body of this media type is written, or `None` when it cannot be text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    Json,
    Form,
    Xml,
    Text,
}

pub fn wire(media_type: &str) -> Option<Wire> {
    let essence = media_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    Some(match essence.as_str() {
        m if m == "application/json" || m.ends_with("+json") || m == "*/*" => Wire::Json,
        "application/x-www-form-urlencoded" => Wire::Form,
        m if m.ends_with("/xml") || m.ends_with("+xml") => Wire::Xml,
        m if m.starts_with("text/") || m == "application/jsonl" || m == "application/x-ndjson" => {
            Wire::Text
        }
        _ => return None,
    })
}

/// The media type a request is generated with: JSON first, then forms, text, XML.
pub fn preferred(content: &[MediaType]) -> Option<&MediaType> {
    let rank = |m: &MediaType| match wire(&m.media_type) {
        Some(Wire::Json) => 0,
        Some(Wire::Form) => 1,
        Some(Wire::Text) => 2,
        Some(Wire::Xml) => 3,
        None => 4,
    };
    content.iter().min_by_key(|m| rank(m))
}

/// A body's text for `value`.
pub fn body_text(wire: Wire, value: &Value) -> String {
    match (wire, value) {
        (Wire::Json, v) => serde_json::to_string_pretty(v).unwrap_or_default(),
        (Wire::Form, v) => form(v),
        (_, Value::String(s)) => s.clone(),
        (Wire::Xml, _) => String::new(),
        (Wire::Text, v) => text_of(v),
    }
}

/// A named example's text as sent: its serialized form, else its value written for `wire`.
pub fn example_text(wire: Wire, example: &NamedExample) -> Option<String> {
    example
        .serialized
        .clone()
        .or_else(|| example.value.as_ref().map(|v| body_text(wire, v)))
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
