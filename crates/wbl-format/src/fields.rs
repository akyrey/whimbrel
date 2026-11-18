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
