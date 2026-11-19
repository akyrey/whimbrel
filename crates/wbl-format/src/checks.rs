//! `[[captures]]` and `[[assertions]]` in a request file (FR-6).

use serde_json::{Map, Number, Value as Json};
use toml_edit::{Array, ArrayOfTables, InlineTable, Item, Table, Value};
use wbl_core::check::{Assertion, Capture, CaptureFrom};

use crate::FormatError;
use crate::examples::{spaced, tables};
use crate::fields::{optional_bool, optional_integer, set, string};

fn from_of(text: &str) -> Result<CaptureFrom, FormatError> {
    match text {
        "body" => Ok(CaptureFrom::Body),
        "header" => Ok(CaptureFrom::Header),
        _ => Err(FormatError::invalid(
            "captures.from",
            "\"body\" or \"header\"",
        )),
    }
}

fn optional_string(t: &Table, key: &str, field: &str) -> Result<Option<String>, FormatError> {
    t.get(key).map(|_| string(t, key, field)).transpose()
}

pub(crate) fn read_captures(root: &Table) -> Result<Vec<Capture>, FormatError> {
    tables(root.get("captures"), "captures")?
        .iter()
        .map(|t| {
            Ok(Capture {
                var: string(t, "var", "captures.var")?,
                from: from_of(&string(t, "from", "captures.from")?)?,
                path: optional_string(t, "path", "captures.path")?.unwrap_or_default(),
                regex: optional_string(t, "regex", "captures.regex")?,
                secret: optional_bool(t, "secret", "captures.secret", false)?,
            })
        })
        .collect()
}

pub(crate) fn write_captures(root: &mut Table, captures: &[Capture]) {
    if read_captures(root).ok().as_deref() == Some(captures) {
        return;
    }
    let mut aot = ArrayOfTables::new();
    for c in captures {
        let mut t = spaced(Table::new());
        set(&mut t, "var", c.var.as_str());
        set(
            &mut t,
            "from",
            if c.from == CaptureFrom::Body {
                "body"
            } else {
                "header"
            },
        );
        set(&mut t, "path", c.path.as_str());
        if let Some(regex) = &c.regex {
            set(&mut t, "regex", regex.as_str());
        }
        if c.secret {
            set(&mut t, "secret", true);
        }
        aot.push(t);
    }
    replace(root, "captures", aot);
}

fn replace(root: &mut Table, key: &str, aot: ArrayOfTables) {
    if aot.is_empty() {
        root.remove(key);
    } else {
        root.insert(key, Item::ArrayOfTables(aot));
    }
}

/// A TOML value as JSON (dates become text).
fn to_json(value: &Value) -> Json {
    match value {
        Value::String(s) => Json::String(s.value().clone()),
        Value::Integer(i) => Json::Number((*i.value()).into()),
        Value::Float(f) => Number::from_f64(*f.value()).map_or(Json::Null, Json::Number),
        Value::Boolean(b) => Json::Bool(*b.value()),
        Value::Datetime(d) => Json::String(d.value().to_string()),
        Value::Array(a) => Json::Array(a.iter().map(to_json).collect()),
        Value::InlineTable(t) => Json::Object(
            t.iter()
                .map(|(k, v)| (k.to_owned(), to_json(v)))
                .collect::<Map<_, _>>(),
        ),
    }
}

/// JSON as a TOML value; `null` (which TOML lacks) is written as the text `"null"`.
fn to_toml(value: &Json) -> Value {
    match value {
        Json::Null => Value::from("null"),
        Json::Bool(b) => Value::from(*b),
        Json::Number(n) => n
            .as_i64()
            .map_or_else(|| Value::from(n.as_f64().unwrap_or_default()), Value::from),
        Json::String(s) => Value::from(s.as_str()),
        Json::Array(items) => Value::Array(items.iter().map(to_toml).collect::<Array>()),
        Json::Object(map) => {
            let mut t = InlineTable::new();
            for (k, v) in map {
                t.insert(k, to_toml(v));
            }
            Value::InlineTable(t)
        }
    }
}

fn number(t: &Table, key: &str, field: &str, what: &str) -> Result<u64, FormatError> {
    optional_integer(t, key, field)?
        .and_then(|n| u64::try_from(n).ok())
        .ok_or_else(|| FormatError::invalid(field, what))
}

fn read_assertion(t: &Table) -> Result<Assertion, FormatError> {
    match string(t, "check", "assertions.check")?.as_str() {
        "status" => {
            let code = number(t, "equals", "assertions.equals", "a status code")?;
            Ok(Assertion::Status {
                equals: u16::try_from(code)
                    .map_err(|_| FormatError::invalid("assertions.equals", "a status code"))?,
            })
        }
        "header" => Ok(Assertion::Header {
            name: string(t, "name", "assertions.name")?,
            equals: optional_string(t, "equals", "assertions.equals")?,
        }),
        "json" => Ok(Assertion::Json {
            path: string(t, "path", "assertions.path")?,
            equals: t.get("equals").and_then(Item::as_value).map(to_json),
        }),
        "time" => Ok(Assertion::Time {
            below_ms: number(t, "below_ms", "assertions.below_ms", "milliseconds")?,
        }),
        _ => Err(FormatError::invalid(
            "assertions.check",
            "status, header, json or time",
        )),
    }
}

pub(crate) fn read_assertions(root: &Table) -> Result<Vec<Assertion>, FormatError> {
    tables(root.get("assertions"), "assertions")?
        .iter()
        .map(read_assertion)
        .collect()
}

fn assertion_table(a: &Assertion) -> Table {
    let mut t = spaced(Table::new());
    match a {
        Assertion::Status { equals } => {
            set(&mut t, "check", "status");
            set(&mut t, "equals", i64::from(*equals));
        }
        Assertion::Header { name, equals } => {
            set(&mut t, "check", "header");
            set(&mut t, "name", name.as_str());
            if let Some(e) = equals {
                set(&mut t, "equals", e.as_str());
            }
        }
        Assertion::Json { path, equals } => {
            set(&mut t, "check", "json");
            set(&mut t, "path", path.as_str());
            if let Some(e) = equals {
                t.insert("equals", Item::Value(to_toml(e)));
            }
        }
        Assertion::Time { below_ms } => {
            set(&mut t, "check", "time");
            set(
                &mut t,
                "below_ms",
                i64::try_from(*below_ms).unwrap_or(i64::MAX),
            );
        }
    }
    t
}

pub(crate) fn write_assertions(root: &mut Table, assertions: &[Assertion]) {
    if read_assertions(root).ok().as_deref() == Some(assertions) {
        return;
    }
    let mut aot = ArrayOfTables::new();
    for a in assertions {
        aot.push(assertion_table(a));
    }
    replace(root, "assertions", aot);
}
