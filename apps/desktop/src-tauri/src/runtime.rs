//! Runtime variables (FR-6.1, D-H17): what captures took from responses, per collection, kept in
//! memory until the app quits. They come after the request's own variables and before the
//! environment.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use serde::Serialize;
use ts_rs::TS;
use wbl_core::interpolate::{Scope, Value};

use crate::error::AppError;

/// The key of requests that are not in a collection.
pub const LOOSE: &str = "";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct RuntimeVariable {
    pub name: String,
    /// `None` for a secret: its value never goes to the frontend.
    pub value: Option<String>,
    pub secret: bool,
}

#[derive(Default)]
pub struct Runtime {
    values: Mutex<HashMap<String, BTreeMap<String, Value>>>,
}

fn poisoned() -> AppError {
    AppError::internal("The runtime variables are unusable; restart the app")
}

impl Runtime {
    pub fn scope(&self, key: &str) -> Result<Scope, AppError> {
        let values = self.values.lock().map_err(|_| poisoned())?;
        Ok(Scope::new(values.get(key).cloned().unwrap_or_default()))
    }

    pub fn set(&self, key: &str, name: &str, value: String, secret: bool) -> Result<(), AppError> {
        let stored = if secret {
            Value::Secret(Some(value))
        } else {
            Value::Plain(value)
        };
        let mut values = self.values.lock().map_err(|_| poisoned())?;
        values
            .entry(key.to_owned())
            .or_default()
            .insert(name.to_owned(), stored);
        Ok(())
    }

    pub fn clear(&self, key: &str) -> Result<(), AppError> {
        self.values.lock().map_err(|_| poisoned())?.remove(key);
        Ok(())
    }

    pub fn list(&self, key: &str) -> Result<Vec<RuntimeVariable>, AppError> {
        let values = self.values.lock().map_err(|_| poisoned())?;
        Ok(values
            .get(key)
            .into_iter()
            .flatten()
            .map(|(name, value)| match value {
                Value::Plain(v) => RuntimeVariable {
                    name: name.clone(),
                    value: Some(v.clone()),
                    secret: false,
                },
                Value::Secret(_) => RuntimeVariable {
                    name: name.clone(),
                    value: None,
                    secret: true,
                },
            })
            .collect())
    }
}

#[derive(Debug, Clone, serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeArgs {
    /// The collection; requests outside one when absent.
    #[ts(optional)]
    pub root: Option<String>,
}

fn key_of(
    collections: &crate::collections::Collections,
    args: &RuntimeArgs,
) -> Result<String, AppError> {
    let root = args
        .root
        .as_deref()
        .map(|r| collections.opened(r))
        .transpose()?;
    Ok(crate::commands::runtime_key(root.as_deref()))
}

pub fn list(
    collections: &crate::collections::Collections,
    args: &RuntimeArgs,
) -> Result<Vec<RuntimeVariable>, AppError> {
    collections.runtime.list(&key_of(collections, args)?)
}

pub fn clear(
    collections: &crate::collections::Collections,
    args: &RuntimeArgs,
) -> Result<(), AppError> {
    collections.runtime.clear(&key_of(collections, args)?)
}

#[tauri::command]
pub async fn runtime_list(
    collections: tauri::State<'_, crate::collections::Collections>,
    args: RuntimeArgs,
) -> Result<Vec<RuntimeVariable>, AppError> {
    crate::logging::logged("runtime_list", async { list(&collections, &args) }).await
}

#[tauri::command]
pub async fn runtime_clear(
    collections: tauri::State<'_, crate::collections::Collections>,
    args: RuntimeArgs,
) -> Result<(), AppError> {
    crate::logging::logged("runtime_clear", async { clear(&collections, &args) }).await
}
