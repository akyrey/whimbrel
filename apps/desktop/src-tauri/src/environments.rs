//! Environment, variable and secret commands (Phase 3). Secret values only go in.

use std::collections::BTreeMap;

use serde::Serialize;
use tauri::State;
use ts_rs::TS;
use wbl_format::collection::environments::{EnvironmentEntry, EnvironmentRead};
use wbl_format::collection::{self, environments};

use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::{
    CollectionArgs, EnvironmentArgs, EnvironmentCreateArgs, EnvironmentSaveArgs, SecretArgs,
    SecretSetArgs, VariablesSaveArgs,
};
use crate::logging::logged;
use crate::secrets::{SecretState, Secrets};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct EnvironmentView {
    #[serde(flatten)]
    pub read: EnvironmentRead,
    /// Which secret names have a value in the keychain.
    pub secrets: Vec<SecretState>,
}

pub fn list(state: &Collections, args: &CollectionArgs) -> Result<Vec<EnvironmentEntry>, AppError> {
    Ok(environments::list(&state.opened(&args.root)?)?)
}

pub fn read(
    state: &Collections,
    secrets: &Secrets,
    args: &EnvironmentArgs,
) -> Result<EnvironmentView, AppError> {
    let root = state.opened(&args.root)?;
    let read = environments::read(&root, &args.id)?;
    let names: Vec<String> = read
        .environment
        .secrets
        .iter()
        .map(|s| s.name.clone())
        .collect();
    let states = secrets.states(&collection::id(&root)?, &args.id, &names)?;
    Ok(EnvironmentView {
        read,
        secrets: states,
    })
}

pub fn create(state: &Collections, args: &EnvironmentCreateArgs) -> Result<String, AppError> {
    Ok(environments::create(
        &state.opened(&args.root)?,
        &args.environment,
    )?)
}

pub fn save(state: &Collections, args: &EnvironmentSaveArgs) -> Result<String, AppError> {
    Ok(environments::save(
        &state.opened(&args.root)?,
        &args.id,
        &args.environment,
        &args.hash,
    )?)
}

/// Deletes the environment's files and its secret values.
pub fn delete(
    state: &Collections,
    secrets: &Secrets,
    args: &EnvironmentArgs,
) -> Result<(), AppError> {
    let root = state.opened(&args.root)?;
    let read = environments::read(&root, &args.id)?;
    let collection_id = collection::id(&root)?;
    for secret in &read.environment.secrets {
        secrets.clear(&collection_id, &args.id, &secret.name)?;
    }
    Ok(environments::delete(&root, &args.id)?)
}

pub fn save_variables(state: &Collections, args: &VariablesSaveArgs) -> Result<(), AppError> {
    let root = state.opened(&args.root)?;
    match &args.id {
        Some(id) => Ok(environments::save_local(&root, id, &args.variables)?),
        None => Ok(collection::save_variables(&root, &args.variables)?),
    }
}

pub fn collection_variables(
    state: &Collections,
    args: &CollectionArgs,
) -> Result<BTreeMap<String, String>, AppError> {
    Ok(collection::variables(&state.opened(&args.root)?)?)
}

/// The secret must be declared by the environment; its value goes to the keychain only.
pub fn set_secret(
    state: &Collections,
    secrets: &Secrets,
    args: &SecretSetArgs,
) -> Result<(), AppError> {
    let root = state.opened(&args.root)?;
    let read = environments::read(&root, &args.environment)?;
    if !read.environment.secrets.iter().any(|s| s.name == args.name) {
        return Err(AppError::new(
            "not_found",
            format!("{} declares no secret {}", read.environment.name, args.name),
        ));
    }
    secrets.set(
        &collection::id(&root)?,
        &args.environment,
        &args.name,
        &args.value,
    )
}

pub fn clear_secret(
    state: &Collections,
    secrets: &Secrets,
    args: &SecretArgs,
) -> Result<(), AppError> {
    let root = state.opened(&args.root)?;
    secrets.clear(&collection::id(&root)?, &args.environment, &args.name)
}

macro_rules! command {
    ($name:ident, $args:ty, $out:ty, $run:path) => {
        #[tauri::command]
        pub async fn $name(state: State<'_, Collections>, args: $args) -> Result<$out, AppError> {
            logged(stringify!($name), async { $run(&state, &args) }).await
        }
    };
    ($name:ident, $args:ty, $out:ty, $run:path, secrets) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, Collections>,
            secrets: State<'_, Secrets>,
            args: $args,
        ) -> Result<$out, AppError> {
            logged(stringify!($name), async { $run(&state, &secrets, &args) }).await
        }
    };
}

command!(
    environment_list,
    CollectionArgs,
    Vec<EnvironmentEntry>,
    list
);
command!(
    environment_read,
    EnvironmentArgs,
    EnvironmentView,
    read,
    secrets
);
command!(environment_create, EnvironmentCreateArgs, String, create);
command!(environment_save, EnvironmentSaveArgs, String, save);
command!(environment_delete, EnvironmentArgs, (), delete, secrets);
command!(variables_save, VariablesSaveArgs, (), save_variables);
command!(collection_variables_read, CollectionArgs, BTreeMap<String, String>, collection_variables);
command!(secret_set, SecretSetArgs, (), set_secret, secrets);
command!(secret_clear, SecretArgs, (), clear_secret, secrets);

#[cfg(test)]
mod tests;
