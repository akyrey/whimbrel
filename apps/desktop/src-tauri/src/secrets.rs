//! Secret values in the OS keychain (D-H04): one entry per collection, environment and name.
//! Values go in (`set`) and are used by the backend; they never go back to the frontend.

use std::sync::Arc;

use roost_keychain::SecretStore;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppError;

/// The keychain service for every Whimbrel secret.
pub const SERVICE: &str = "whimbrel";

pub struct Secrets(Arc<dyn SecretStore>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct SecretState {
    pub name: String,
    /// Whether the keychain holds a value for it. The value itself never leaves the backend.
    pub set: bool,
}

fn account(collection_id: &str, environment: &str, name: &str) -> String {
    format!("{collection_id}:{environment}:{name}")
}

fn keychain(error: roost_keychain::KeychainError) -> AppError {
    AppError::new("keychain", error.0)
}

impl Secrets {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self(store)
    }

    pub fn set(
        &self,
        collection: &str,
        environment: &str,
        name: &str,
        value: &str,
    ) -> Result<(), AppError> {
        let secret = SecretString::from(value.to_owned());
        self.0
            .set(&account(collection, environment, name), &secret)
            .map_err(keychain)
    }

    pub fn clear(&self, collection: &str, environment: &str, name: &str) -> Result<(), AppError> {
        self.0
            .delete(&account(collection, environment, name))
            .map_err(keychain)
    }

    /// The value, for resolving a request; `None` when not set.
    pub fn get(
        &self,
        collection: &str,
        environment: &str,
        name: &str,
    ) -> Result<Option<String>, AppError> {
        Ok(self
            .0
            .get(&account(collection, environment, name))
            .map_err(keychain)?
            .map(|s| s.expose_secret().to_owned()))
    }

    pub fn states(
        &self,
        collection: &str,
        environment: &str,
        names: &[String],
    ) -> Result<Vec<SecretState>, AppError> {
        names
            .iter()
            .map(|name| {
                Ok(SecretState {
                    name: name.clone(),
                    set: self.get(collection, environment, name)?.is_some(),
                })
            })
            .collect()
    }
}
