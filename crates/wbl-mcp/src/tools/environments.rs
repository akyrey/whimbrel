//! Changing environments and collection variables (task 12.6). Secrets are names only: no tool
//! sets, shows or clears a secret value (D-H04).

use std::collections::BTreeMap;
use std::path::Path;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use wbl_format::collection::{self, environments};
use wbl_format::{Environment, SecretName};

use crate::error::McpError;
use crate::server::{Server, reply};
use crate::tools::read::{EnvironmentArgs, environment};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateEnvironmentArgs {
    pub collection: String,
    pub name: String,
    /// Plain variables, written to the environment file in git.
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    /// Names of secret variables. Their values are set by the user in the Whimbrel app and
    /// kept in the OS keychain.
    #[serde(default)]
    pub secrets: Vec<String>,
    /// Sending with a protected environment needs the app's confirmation, so this server
    /// refuses to send with it.
    #[serde(default)]
    pub protected: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVariablesArgs {
    pub collection: String,
    /// The environment's id, as list_environments gives it.
    pub id: String,
    /// Plain variables to add or change. A secret of the same name becomes plain.
    #[serde(default)]
    pub set: BTreeMap<String, String>,
    /// Variables to make secret: the name is kept, any value is removed from the file, and
    /// the user sets the value in the app.
    #[serde(default)]
    pub secret: Vec<String>,
    /// Variables (plain or secret) to remove.
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CollectionVariablesArgs {
    pub collection: String,
    /// Variables to add or change: the lowest scope, shared by every environment.
    #[serde(default)]
    pub set: BTreeMap<String, String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

fn secret_name(name: &str) -> SecretName {
    SecretName {
        name: name.to_owned(),
        description: String::new(),
    }
}

/// `environment` with the changes of `args` applied.
fn changed(environment: &Environment, args: &SetVariablesArgs) -> Environment {
    let dropped = |name: &str| {
        args.remove.iter().any(|r| r == name)
            || args.secret.iter().any(|s| s == name)
            || args.set.contains_key(name)
    };
    let mut variables: BTreeMap<String, String> = environment
        .variables
        .iter()
        .filter(|(name, _)| !dropped(name))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    variables.extend(args.set.clone());
    let mut secrets: Vec<SecretName> = environment
        .secrets
        .iter()
        .filter(|s| !args.remove.contains(&s.name) && !args.set.contains_key(&s.name))
        .cloned()
        .collect();
    for name in &args.secret {
        if !secrets.iter().any(|s| s.name == *name) && !args.remove.contains(name) {
            secrets.push(secret_name(name));
        }
    }
    Environment {
        variables,
        secrets,
        ..environment.clone()
    }
}

fn guard_values<'a>(
    server: &Server,
    root: &Path,
    values: impl IntoIterator<Item = &'a String>,
) -> Result<(), McpError> {
    let text: Vec<&str> = values.into_iter().map(String::as_str).collect();
    Ok(wbl_engine::resolve::guard(
        root,
        &text.join("\n"),
        &server.state.secrets,
    )?)
}

impl Server {
    fn create_environment_in(&self, args: CreateEnvironmentArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        guard_values(self, &root, args.variables.values())?;
        let id = environments::create(
            &root,
            &Environment {
                name: args.name,
                protected: args.protected,
                variables: args.variables,
                secrets: args.secrets.iter().map(|s| secret_name(s)).collect(),
            },
        )?;
        environment(self, &root, &id)
    }

    fn set_variables_in(&self, args: SetVariablesArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        guard_values(self, &root, args.set.values())?;
        let read = environments::read(&root, &args.id)?;
        environments::save(
            &root,
            &args.id,
            &changed(&read.environment, &args),
            &read.hash,
        )?;
        environment(self, &root, &args.id)
    }

    fn collection_variables_in(&self, args: CollectionVariablesArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        guard_values(self, &root, args.set.values())?;
        let mut variables = collection::variables(&root)?;
        variables.retain(|name, _| !args.remove.contains(name));
        variables.extend(args.set);
        collection::save_variables(&root, &variables)?;
        Ok(json!({ "variables": collection::variables(&root)? }))
    }
}

#[tool_router(router = environment_tools, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "Creates an environment with plain variables and secret names, and \
returns it with its id. Secret values are set by the user in the Whimbrel app."
    )]
    async fn create_environment(
        &self,
        Parameters(args): Parameters<CreateEnvironmentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.create_environment_in(args)))
    }

    #[tool(
        description = "Adds, changes, removes or makes secret an environment's variables. \
No value of a secret is ever set or shown here; literal secret values are refused."
    )]
    async fn set_variables(
        &self,
        Parameters(args): Parameters<SetVariablesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.set_variables_in(args)))
    }

    #[tool(
        description = "Adds, changes or removes the collection's own variables, the lowest \
scope, used by every environment."
    )]
    async fn set_collection_variables(
        &self,
        Parameters(args): Parameters<CollectionVariablesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.collection_variables_in(args)))
    }

    #[tool(
        description = "Deletes an environment's file and the user's .local values for it. \
Its secret values stay in the keychain, so restoring the file from git brings them back."
    )]
    async fn delete_environment(
        &self,
        Parameters(args): Parameters<EnvironmentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|root| {
            environments::delete(&root, &args.id)?;
            Ok(json!({ "deleted": args.id }))
        })))
    }
}

#[cfg(test)]
mod tests;
