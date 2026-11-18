//! Reading typed values from a TOML table, and writing a value only when it changed, keeping
//! the comment and spacing around it.

use toml_edit::{Item, Table, Value};

use crate::FormatError;

pub(crate) fn string(table: &Table, key: &str, field: &str) -> Result<String, FormatError> {
    match table.get(key) {
        None => Err(FormatError::Missing(field.to_owned())),
        Some(item) => item
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| FormatError::invalid(field, "text")),
    }
}

pub(crate) fn optional_bool(
    table: &Table,
    key: &str,
    field: &str,
    default: bool,
) -> Result<bool, FormatError> {
    match table.get(key) {
        None => Ok(default),
        Some(item) => item
            .as_bool()
            .ok_or_else(|| FormatError::invalid(field, "true or false")),
    }
}

pub(crate) fn optional_integer(
    table: &Table,
    key: &str,
    field: &str,
) -> Result<Option<i64>, FormatError> {
    match table.get(key) {
        None => Ok(None),
        Some(item) => item
            .as_integer()
            .map(Some)
            .ok_or_else(|| FormatError::invalid(field, "a whole number")),
    }
}

/// Sets `key` to `value` unless it already holds an equal value; a replaced value keeps the old
/// one's decor (the spaces and comment around it).
pub(crate) fn set(table: &mut Table, key: &str, value: impl Into<Value>) {
    let mut value = value.into();
    match table.get_mut(key) {
        Some(Item::Value(current)) => {
            if same(current, &value) {
                return;
            }
            *value.decor_mut() = current.decor().clone();
            *current = value;
        }
        _ => {
            table.insert(key, Item::Value(value));
        }
    }
}

fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x.value() == y.value(),
        (Value::Integer(x), Value::Integer(y)) => x.value() == y.value(),
        (Value::Boolean(x), Value::Boolean(y)) => x.value() == y.value(),
        _ => false,
    }
}

/// A table of text values (`[variables]`), or an empty map when absent.
pub(crate) fn string_map(
    table: &Table,
    key: &str,
) -> Result<std::collections::BTreeMap<String, String>, FormatError> {
    let Some(item) = table.get(key) else {
        return Ok(std::collections::BTreeMap::new());
    };
    let map = item
        .as_table_like()
        .ok_or_else(|| FormatError::invalid(key, "a table of text values"))?;
    map.iter()
        .map(|(name, value)| {
            value
                .as_str()
                .map(|v| (name.to_owned(), v.to_owned()))
                .ok_or_else(|| FormatError::invalid(&format!("{key}.{name}"), "text"))
        })
        .collect()
}

/// Writes `map` as the table `key`: unchanged values (and their comments) stay, removed keys go,
/// new ones are added; an empty map removes the table.
pub(crate) fn set_string_map(
    table: &mut Table,
    key: &str,
    map: &std::collections::BTreeMap<String, String>,
) {
    if map.is_empty() {
        table.remove(key);
        return;
    }
    if !matches!(table.get(key), Some(Item::Table(_))) {
        let mut fresh = Table::new();
        fresh.decor_mut().set_prefix("\n");
        table.insert(key, Item::Table(fresh));
    }
    let Some(Item::Table(inner)) = table.get_mut(key) else {
        return;
    };
    let stale: Vec<String> = inner
        .iter()
        .map(|(k, _)| k.to_owned())
        .filter(|k| !map.contains_key(k))
        .collect();
    for k in stale {
        inner.remove(&k);
    }
    for (k, v) in map {
        set(inner, k, v.as_str());
    }
}
