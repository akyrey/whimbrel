//! Reading collections and environments (task 12.4). Nothing here writes a file.

use std::path::Path;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wbl_format::collection::{self, auth, environments, headers};

use crate::error::McpError;
use crate::server::{Server, reply};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CollectionArgs {
    /// The collection's folder or name, as list_collections gives them.
    pub collection: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NodeArgs {
    /// The collection's folder or name, as list_collections gives them.
    pub collection: String,
    /// The request file or folder in the collection, as get_tree gives it (relative, with '/').
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvironmentArgs {
    /// The collection's folder or name, as list_collections gives them.
    pub collection: String,
    /// The environment's id, as list_environments gives it.
    pub id: String,
}

#[derive(Debug, Serialize)]
struct SecretView {
    name: String,
    description: String,
    /// Whether the keychain holds a value. The value itself is never shown.
    set: bool,
}

fn tree(root: &Path) -> Result<Value, McpError> {
    let loaded = collection::load(root)?;
    Ok(json!({
        "id": loaded.id,
        "name": loaded.name,
        "variables": collection::variables(root)?,
        "headers": headers::headers_of(root, "")?,
        "auth": auth::auth_of(root, "")?,
        "children": loaded.children,
    }))
}

fn request(root: &Path, path: &str) -> Result<Value, McpError> {
    let read = collection::read_request(root, path)?;
    let chain = headers::headers_chain(root, path)?;
    let auths = auth::auth_chain(root, path)?;
    let mut links = vec![&read.stored.auth];
    links.extend(auths.iter());
    let sent_headers = wbl_core::inherit::with_inherited(&read.stored.request, &chain).headers;
    Ok(json!({
        "path": path,
        "request": read.stored,
        "sentHeaders": sent_headers,
        "effectiveAuth": wbl_core::auth::effective(&links),
    }))
}

fn environment(server: &Server, root: &Path, id: &str) -> Result<Value, McpError> {
    let read = environments::read(root, id)?;
    let collection_id = collection::id(root)?;
    let secrets = read
        .environment
        .secrets
        .iter()
        .map(|s| {
            Ok(SecretView {
                name: s.name.clone(),
                description: s.description.clone(),
                set: server
                    .state
                    .secrets
                    .get(&collection_id, id, &s.name)?
                    .is_some(),
            })
        })
        .collect::<Result<Vec<_>, McpError>>()?;
    Ok(json!({
        "id": read.id,
        "name": read.environment.name,
        "protected": read.environment.protected,
        "variables": read.environment.variables,
        "local": read.local,
        "secrets": secrets,
    }))
}

#[derive(Debug, Serialize)]
struct CollectionSummary {
    name: String,
    folder: String,
}

#[tool_router(router = read_tools, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "The collections this server can read and change: name and folder. \
Every other tool names one of them as `collection`."
    )]
    async fn list_collections(&self) -> Result<CallToolResult, ErrorData> {
        let list: Vec<CollectionSummary> = self
            .state
            .collections
            .list()
            .iter()
            .map(|c| CollectionSummary {
                name: c.name.clone(),
                folder: c.root.to_string_lossy().into_owned(),
            })
            .collect();
        Ok(reply(Ok::<_, McpError>(json!({ "collections": list }))))
    }

    #[tool(
        description = "A collection's tree (folders and requests with method, name, path and \
order, in the order shown), its variables, and the headers and auth its requests inherit. Files \
that cannot be read are listed as `broken` with the reason."
    )]
    async fn get_tree(
        &self,
        Parameters(args): Parameters<CollectionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|r| tree(&r))))
    }

    #[tool(
        description = "One request file: the request with its examples, settings, auth, \
variables, captures and assertions (the shape update_request takes back), the headers it sends \
once folder and collection headers are inherited, and the auth that applies."
    )]
    async fn get_request(
        &self,
        Parameters(args): Parameters<NodeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(
            self.root(&args.collection)
                .and_then(|r| request(&r, &args.path)),
        ))
    }

    #[tool(
        description = "A collection's environments: id, name, whether protected, and why a \
file cannot be read."
    )]
    async fn list_environments(
        &self,
        Parameters(args): Parameters<CollectionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|r| {
            Ok(json!({ "environments": environments::list(&r)? }))
        })))
    }

    #[tool(
        description = "An environment: its variables, the user's own values from .local \
(not in git), and its secrets by name with whether each is set. Secret values are never shown."
    )]
    async fn get_environment(
        &self,
        Parameters(args): Parameters<EnvironmentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(
            self.root(&args.collection)
                .and_then(|r| environment(self, &r, &args.id)),
        ))
    }
}

#[cfg(test)]
mod tests;
