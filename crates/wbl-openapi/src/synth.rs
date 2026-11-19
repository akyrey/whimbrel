//! Example values from JSON Schema 2020-12 (task 6.4): what the schema says first (`const`,
//! `examples`, `default`, `enum`), else a value of its type. Cycles and depth are bounded.

use std::collections::BTreeMap;

use serde_json::{Map, Number, Value, json};

/// Nesting beyond this yields `null` (or leaves an optional property out).
const MAX_DEPTH: usize = 8;

/// Which side the value is for: requests leave out `readOnly`, responses `writeOnly`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Request,
    Response,
}

pub struct Synth<'a> {
    schemas: &'a BTreeMap<String, Value>,
    direction: Direction,
    /// Schema names being expanded, to stop at a cycle.
    stack: Vec<String>,
}

/// An example of `schema`, whose `$ref`s point into `schemas` (`#/schemas/<name>`).
pub fn example(schema: &Value, schemas: &BTreeMap<String, Value>, direction: Direction) -> Value {
    Synth {
        schemas,
        direction,
        stack: Vec::new(),
    }
    .value(schema, 0)
    .unwrap_or(Value::Null)
}

fn string_for(format: &str) -> &'static str {
    match format {
        "date-time" => "2024-01-01T12:00:00Z",
        "date" => "2024-01-01",
        "time" => "12:00:00Z",
        "duration" => "P1D",
        "email" | "idn-email" => "user@example.com",
        "hostname" | "idn-hostname" => "example.com",
        "ipv4" => "192.0.2.1",
        "ipv6" => "2001:db8::1",
        "uri" | "url" | "iri" | "uri-reference" | "iri-reference" => "https://example.com",
        "uuid" => "3fa85f64-5717-4562-b3fc-2c963f66afa6",
        "byte" => "aGVsbG8=",
        "binary" => "",
        "password" => "password",
        "regex" => ".*",
        "json-pointer" => "/a",
        _ => "string",
    }
}

fn the_type(schema: &Map<String, Value>) -> Option<String> {
    match schema.get("type") {
        Some(Value::String(t)) => Some(t.clone()),
        // `["string", "null"]`: the first that is not null.
        Some(Value::Array(types)) => types
            .iter()
            .filter_map(Value::as_str)
            .find(|t| *t != "null")
            .or_else(|| types.first().and_then(Value::as_str))
            .map(str::to_owned),
        _ if schema.contains_key("properties") || schema.contains_key("additionalProperties") => {
            Some("object".into())
        }
        _ if schema.contains_key("items") || schema.contains_key("prefixItems") => {
            Some("array".into())
        }
        _ => None,
    }
}

/// Merges `b` into `a` (objects key by key; otherwise `b` wins unless it is null).
fn merge(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Object(mut a), Value::Object(b)) => {
            for (k, v) in b {
                let merged = match a.remove(&k) {
                    Some(old) => merge(old, v),
                    None => v,
                };
                a.insert(k, merged);
            }
            Value::Object(a)
        }
        (a, Value::Null) => a,
        (_, b) => b,
    }
}

impl Synth<'_> {
    /// `None`: nothing sensible (a cycle or too deep).
    fn value(&mut self, schema: &Value, depth: usize) -> Option<Value> {
        if depth > MAX_DEPTH {
            return None;
        }
        let schema = match schema {
            Value::Object(map) => map,
            Value::Bool(true) => return Some(json!({})),
            _ => return Some(Value::Null),
        };
        if let Some(found) = Self::stated(schema) {
            return Some(found);
        }
        if let Some(Value::String(reference)) = schema.get("$ref") {
            return self.reference(reference, schema, depth);
        }
        if let Some(Value::Array(parts)) = schema.get("allOf") {
            let base = self.typed(schema, depth).unwrap_or(Value::Null);
            return Some(
                parts
                    .iter()
                    .filter_map(|p| self.value(p, depth + 1))
                    .fold(base, merge),
            );
        }
        for key in ["oneOf", "anyOf"] {
            if let Some(Value::Array(choices)) = schema.get(key) {
                let first = choices.iter().find_map(|c| self.value(c, depth + 1));
                let own = self.typed(schema, depth).unwrap_or(Value::Null);
                return first.map(|f| merge(own, f));
            }
        }
        self.typed(schema, depth)
    }

    /// A value the schema gives itself.
    fn stated(schema: &Map<String, Value>) -> Option<Value> {
        if let Some(v) = schema.get("const") {
            return Some(v.clone());
        }
        if let Some(v) = schema
            .get("examples")
            .and_then(Value::as_array)
            .and_then(|e| e.first())
        {
            return Some(v.clone());
        }
        // 3.0 schemas that were not rewritten (`example` inside a parameter's schema).
        if let Some(v) = schema.get("example").or_else(|| schema.get("default")) {
            return Some(v.clone());
        }
        schema
            .get("enum")
            .and_then(Value::as_array)
            .and_then(|e| e.first())
            .cloned()
    }

    fn reference(
        &mut self,
        reference: &str,
        schema: &Map<String, Value>,
        depth: usize,
    ) -> Option<Value> {
        let name = reference.strip_prefix("#/schemas/")?.to_owned();
        if self.stack.contains(&name) {
            return None;
        }
        let target = self.schemas.get(&name)?;
        self.stack.push(name);
        let value = self.value(target, depth + 1);
        self.stack.pop();
        // Keywords beside `$ref` (3.1) apply too.
        let siblings: Map<String, Value> = schema
            .iter()
            .filter(|(k, _)| *k != "$ref")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        match (value, siblings.is_empty()) {
            (value, true) => value,
            (value, false) => Some(merge(
                value.unwrap_or(Value::Null),
                self.typed(&siblings, depth).unwrap_or(Value::Null),
            )),
        }
    }

    fn typed(&mut self, schema: &Map<String, Value>, depth: usize) -> Option<Value> {
        let format = schema.get("format").and_then(Value::as_str).unwrap_or("");
        Some(match the_type(schema).as_deref() {
            Some("object") => self.object(schema, depth),
            Some("array") => self.array(schema, depth)?,
            Some("string") => Value::String(Self::string(schema, format)),
            Some("integer") => json!(Self::bound(schema).map_or(0, |n| n.ceil() as i64)),
            Some("number") => Self::bound(schema)
                .and_then(Number::from_f64)
                .map_or(json!(0), Value::Number),
            Some("boolean") => json!(true),
            Some("null") | None => Value::Null,
            Some(_) => Value::Null,
        })
    }

    fn string(schema: &Map<String, Value>, format: &str) -> String {
        let text = string_for(format);
        let min = schema.get("minLength").and_then(Value::as_u64).unwrap_or(0);
        let min = usize::try_from(min).unwrap_or(0).min(64);
        if text.len() >= min {
            text.to_owned()
        } else {
            format!("{text}{}", "x".repeat(min - text.len()))
        }
    }

    fn bound(schema: &Map<String, Value>) -> Option<f64> {
        let min = schema.get("minimum").and_then(Value::as_f64);
        let exclusive = schema
            .get("exclusiveMinimum")
            .and_then(Value::as_f64)
            .map(|m| m + 1.0);
        min.or(exclusive)
    }

    fn skipped(&self, property: &Value) -> bool {
        let flag = |k| property.get(k).and_then(Value::as_bool).unwrap_or(false);
        match self.direction {
            Direction::Request => flag("readOnly"),
            Direction::Response => flag("writeOnly"),
        }
    }

    fn object(&mut self, schema: &Map<String, Value>, depth: usize) -> Value {
        let required: Vec<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let mut out = Map::new();
        for (name, property) in schema
            .get("properties")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            if self.skipped(property) {
                continue;
            }
            match self.value(property, depth + 1) {
                Some(v) => {
                    out.insert(name.clone(), v);
                }
                None if required.contains(&name.as_str()) => {
                    out.insert(name.clone(), json!({}));
                }
                None => {}
            }
        }
        if out.is_empty()
            && let Some(extra @ Value::Object(_)) = schema.get("additionalProperties")
            && let Some(v) = self.value(extra, depth + 1)
        {
            out.insert("key".into(), v);
        }
        Value::Object(out)
    }

    fn array(&mut self, schema: &Map<String, Value>, depth: usize) -> Option<Value> {
        if let Some(Value::Array(prefix)) = schema.get("prefixItems") {
            return Some(Value::Array(
                prefix
                    .iter()
                    .map(|p| self.value(p, depth + 1).unwrap_or(Value::Null))
                    .collect(),
            ));
        }
        match schema.get("items") {
            Some(items @ Value::Object(_)) => Some(Value::Array(
                self.value(items, depth + 1).into_iter().collect(),
            )),
            _ => Some(json!([])),
        }
    }
}
