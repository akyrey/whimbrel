//! A response checked against its operation's schema (FR-7.6, D-H17).

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::model::{Api, MediaType, ResponseSpec};

const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
const MODEL_REF: &str = "#/schemas/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SchemaError {
    /// JSON pointer into the body, e.g. `/items/0/id` (`""`: the body itself).
    pub pointer: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Validation {
    /// The response of the spec used: `200`, `2XX` or `default`.
    pub response: Option<String>,
    pub media_type: Option<String>,
    /// A schema was found and the body was checked against it.
    pub checked: bool,
    pub errors: Vec<SchemaError>,
    /// Why nothing was checked.
    pub note: Option<String>,
}

fn noted(note: &str) -> Validation {
    Validation {
        note: Some(note.to_owned()),
        ..Validation::default()
    }
}

/// The spec's response for `status`: exact, then its range (`4XX`), then `default`.
fn response_for(responses: &[ResponseSpec], status: u16) -> Option<&ResponseSpec> {
    let exact = status.to_string();
    let range = format!("{}XX", status / 100);
    responses
        .iter()
        .find(|r| r.status == exact)
        .or_else(|| {
            responses
                .iter()
                .find(|r| r.status.eq_ignore_ascii_case(&range))
        })
        .or_else(|| responses.iter().find(|r| r.status == "default"))
}

fn essence(media_type: &str) -> String {
    media_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

/// The media type entry for `content_type`: exact, then `type/*`, then `*/*`, then the only one.
fn media_for<'a>(content: &'a [MediaType], content_type: Option<&str>) -> Option<&'a MediaType> {
    let wanted = content_type.map(essence).unwrap_or_default();
    let family = wanted
        .split('/')
        .next()
        .map(|t| format!("{t}/*"))
        .unwrap_or_default();
    content
        .iter()
        .find(|m| essence(&m.media_type) == wanted)
        .or_else(|| content.iter().find(|m| essence(&m.media_type) == family))
        .or_else(|| content.iter().find(|m| m.media_type == "*/*"))
        .or_else(|| (content.len() == 1).then(|| &content[0]))
}

/// `#/schemas/X` → `#/$defs/X`, everywhere in `value`.
fn to_defs(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| match (k.as_str(), v) {
                    ("$ref", Value::String(r)) if r.starts_with(MODEL_REF) => (
                        k.clone(),
                        Value::String(format!("#/$defs/{}", &r[MODEL_REF.len()..])),
                    ),
                    _ => (k.clone(), to_defs(v)),
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(to_defs).collect()),
        other => other.clone(),
    }
}

/// The schema as a standalone 2020-12 document, with the model's schema table as `$defs`.
fn standalone(api: &Api, schema: &Value) -> Value {
    let defs: Map<String, Value> = api
        .schemas
        .iter()
        .map(|(k, v)| (k.clone(), to_defs(v)))
        .collect();
    let mut root = match to_defs(schema) {
        Value::Object(map) => map,
        Value::Bool(b) => return Value::Bool(b),
        _ => Map::new(),
    };
    root.insert("$schema".into(), json!(DIALECT));
    root.insert("$defs".into(), Value::Object(defs));
    Value::Object(root)
}

/// Checks `body` (a response of `operation_id` with `status` and `content_type`) against the
/// spec. Only JSON bodies are checked.
pub fn validate(
    api: &Api,
    operation_id: &str,
    status: u16,
    content_type: Option<&str>,
    body: &str,
) -> Validation {
    let Some(op) = api.operations.iter().find(|o| o.id == operation_id) else {
        return noted("The operation is no longer in the spec");
    };
    let Some(response) = response_for(&op.responses, status) else {
        return noted(&format!("The spec does not describe a {status} response"));
    };
    let base = Validation {
        response: Some(response.status.clone()),
        ..Validation::default()
    };
    let Some(media) = media_for(&response.content, content_type) else {
        let note = if response.content.is_empty() {
            "The spec gives this response no body"
        } else {
            "The spec does not describe this content type"
        };
        return Validation {
            note: Some(note.into()),
            ..base
        };
    };
    let base = Validation {
        media_type: Some(media.media_type.clone()),
        ..base
    };
    let Some(schema) = &media.schema else {
        return Validation {
            note: Some("The spec gives this body no schema".into()),
            ..base
        };
    };
    let Ok(instance) = serde_json::from_str::<Value>(body) else {
        return Validation {
            note: Some("The body is not JSON; only JSON bodies are checked".into()),
            ..base
        };
    };
    let validator = match jsonschema::validator_for(&standalone(api, schema)) {
        Ok(v) => v,
        Err(e) => {
            return Validation {
                note: Some(format!("The spec's schema cannot be used: {e}")),
                ..base
            };
        }
    };
    let errors = validator
        .iter_errors(&instance)
        .map(|e| SchemaError {
            pointer: e.instance_path().to_string(),
            message: e.to_string(),
        })
        .collect();
    Validation {
        checked: true,
        errors,
        ..base
    }
}
