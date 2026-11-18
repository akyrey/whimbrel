//! `environments/<id>.toml` (FILE_FORMAT.md, D-H04): variables in the file, secrets by name only.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, InlineTable, Item, Table, Value};

use crate::fields::{optional_bool, optional_integer, set, set_string_map, string, string_map};
use crate::{FORMAT, FormatError};

/// A secret the environment uses: its name, and where to find its value. The value is in the
/// keychain, never here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SecretName {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Environment {
    pub name: String,
    /// Sending with it asks first (FR-4.6).
    pub protected: bool,
    pub variables: BTreeMap<String, String>,
    pub secrets: Vec<SecretName>,
}

#[derive(Debug, Clone)]
pub struct EnvironmentFile {
    doc: DocumentMut,
}

fn read_secrets(root: &Table) -> Result<Vec<SecretName>, FormatError> {
    let Some(item) = root.get("secrets") else {
        return Ok(Vec::new());
    };
    let table = item
        .as_table_like()
        .ok_or_else(|| FormatError::invalid("secrets", "a [secrets] table"))?;
    table
        .iter()
        .map(|(name, value)| {
            let field = format!("secrets.{name}");
            let spec = value.as_table_like().ok_or_else(|| {
                FormatError::invalid(
                    &field,
                    "a table with a description; values belong in the keychain",
                )
            })?;
            let description = spec
                .get("description")
                .and_then(Item::as_str)
                .unwrap_or("")
                .to_owned();
            Ok(SecretName {
                name: name.to_owned(),
                description,
            })
        })
        .collect()
}

fn write_secrets(root: &mut Table, secrets: &[SecretName]) {
    if read_secrets(root).ok().as_deref() == Some(secrets) {
        return;
    }
    if secrets.is_empty() {
        root.remove("secrets");
        return;
    }
    let mut table = crate::examples::spaced(Table::new());
    for secret in secrets {
        let mut spec = InlineTable::new();
        if !secret.description.is_empty() {
            spec.insert("description", Value::from(secret.description.as_str()));
        }
        table.insert(&secret.name, Item::Value(Value::InlineTable(spec)));
    }
    root.insert("secrets", Item::Table(table));
}

impl EnvironmentFile {
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        let doc = text
            .parse::<DocumentMut>()
            .map_err(|e| FormatError::syntax(&e))?;
        Ok(Self { doc })
    }

    pub fn new(environment: &Environment) -> Self {
        let mut doc = DocumentMut::new();
        set(doc.as_table_mut(), "format", FORMAT);
        Self { doc }.updated(environment)
    }

    pub fn read(&self) -> Result<Environment, FormatError> {
        let root = self.doc.as_table();
        let format = optional_integer(root, "format", "format")?.unwrap_or(FORMAT);
        if format > FORMAT {
            return Err(FormatError::TooNew {
                found: format,
                supported: FORMAT,
            });
        }
        Ok(Environment {
            name: string(root, "name", "name")?,
            protected: optional_bool(root, "protected", "protected", false)?,
            variables: string_map(root, "variables")?,
            secrets: read_secrets(root)?,
        })
    }

    pub fn updated(&self, environment: &Environment) -> Self {
        let mut doc = self.doc.clone();
        let root = doc.as_table_mut();
        set(root, "name", environment.name.as_str());
        if environment.protected {
            set(root, "protected", true);
        } else {
            root.remove("protected");
        }
        set_string_map(root, "variables", &environment.variables);
        write_secrets(root, &environment.secrets);
        Self { doc }
    }
}

impl std::fmt::Display for EnvironmentFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.doc)
    }
}

/// `.local/<id>.toml`: the user's own values, git-ignored.
pub fn read_local(text: &str) -> Result<BTreeMap<String, String>, FormatError> {
    let doc = text
        .parse::<DocumentMut>()
        .map_err(|e| FormatError::syntax(&e))?;
    string_map(doc.as_table(), "variables")
}

pub fn write_local(
    text: &str,
    variables: &BTreeMap<String, String>,
) -> Result<String, FormatError> {
    let mut doc = text
        .parse::<DocumentMut>()
        .map_err(|e| FormatError::syntax(&e))?;
    set_string_map(doc.as_table_mut(), "variables", variables);
    Ok(doc.to_string())
}
