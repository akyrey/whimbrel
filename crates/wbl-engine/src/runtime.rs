//! Runtime variables (FR-6.1, D-H17): what captures took from responses, per collection, kept in
//! memory until the app quits. They come after the request's own variables and before the
//! environment.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use serde::Serialize;
use wbl_core::interpolate::{Scope, Value};

use crate::error::EngineError;

/// The key of requests that are not in a collection.
pub const LOOSE: &str = "";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

fn poisoned() -> EngineError {
    EngineError::internal("The runtime variables are unusable; restart the app")
}

impl Runtime {
    pub fn scope(&self, key: &str) -> Result<Scope, EngineError> {
        let values = self.values.lock().map_err(|_| poisoned())?;
        Ok(Scope::new(values.get(key).cloned().unwrap_or_default()))
    }

    pub fn set(
        &self,
        key: &str,
        name: &str,
        value: String,
        secret: bool,
    ) -> Result<(), EngineError> {
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

    pub fn clear(&self, key: &str) -> Result<(), EngineError> {
        self.values.lock().map_err(|_| poisoned())?.remove(key);
        Ok(())
    }

    pub fn list(&self, key: &str) -> Result<Vec<RuntimeVariable>, EngineError> {
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

/// The runtime variables of a collection are keyed by its canonical root.
pub fn runtime_key(root: Option<&std::path::Path>) -> String {
    root.map_or_else(|| LOOSE.to_owned(), |r| r.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn values_are_kept_per_collection_and_secrets_are_not_listed() {
        let runtime = Runtime::default();
        runtime.set("a", "id", "7".into(), false).unwrap();
        runtime.set("a", "token", "t".into(), true).unwrap();
        runtime.set("b", "id", "9".into(), false).unwrap();

        let listed = runtime.list("a").unwrap();
        runtime.clear("a").unwrap();

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].value.as_deref(), Some("7"));
        assert!(listed[1].secret && listed[1].value.is_none());
        assert!(runtime.list("a").unwrap().is_empty());
        assert_eq!(runtime.list("b").unwrap().len(), 1);
        assert_eq!(
            runtime.scope("a").unwrap().entries().count(),
            0,
            "a cleared collection has no runtime scope"
        );
    }

    #[test]
    fn requests_outside_a_collection_share_one_key() {
        assert_eq!(runtime_key(None), LOOSE);
        assert_eq!(runtime_key(Some(std::path::Path::new("/c"))), "/c");
    }
}
