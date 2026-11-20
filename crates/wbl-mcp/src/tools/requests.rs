//! Changing requests and folders (task 12.5), through `wbl-format` as the app does: canonical
//! files, unique names, and literal secret values refused.

use std::path::Path;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use wbl_format::StoredRequest;
use wbl_format::collection::{self, auth, headers};

use crate::error::McpError;
use crate::server::{Server, reply};

/// What a request file holds when a field is not given.
fn blank() -> Value {
    json!({
        "name": "New request",
        "order": 0,
        "request": { "method": "GET", "url": "", "headers": [], "body": { "type": "none" } },
        "examples": [],
        "settings": wbl_core::RequestSettings::default(),
        "auth": { "type": "inherit" },
        "variables": {},
        "captures": [],
        "assertions": [],
    })
}

/// `base` with the fields of `given` laid over it, one level into `request`; headers without
/// `enabled` are enabled.
fn overlay(base: Value, given: Value) -> Result<StoredRequest, McpError> {
    let (Value::Object(mut out), Value::Object(given)) = (base, given) else {
        return Err(McpError::InvalidArgument(
            "`request` must be an object".into(),
        ));
    };
    for (key, value) in given {
        let merged = match (key.as_str(), out.remove(&key), value) {
            ("request", Some(Value::Object(mut inner)), Value::Object(fields)) => {
                inner.extend(fields);
                Value::Object(inner)
            }
            (_, _, value) => value,
        };
        out.insert(key, merged);
    }
    if let Some(Value::Array(list)) = out.get_mut("request").and_then(|r| r.get_mut("headers")) {
        for header in list.iter_mut().filter_map(Value::as_object_mut) {
            header.entry("enabled").or_insert(Value::Bool(true));
        }
    }
    serde_json::from_value(Value::Object(out))
        .map_err(|e| McpError::InvalidArgument(format!("Not a request: {e}")))
}

fn guarded(server: &Server, root: &Path, stored: &StoredRequest) -> Result<(), McpError> {
    let text = wbl_format::RequestFile::new(stored).to_string();
    Ok(wbl_engine::resolve::guard(
        root,
        &text,
        &server.state.secrets,
    )?)
}

fn saved(root: &Path, path: &str) -> Result<Value, McpError> {
    let read = collection::read_request(root, path)?;
    Ok(json!({ "path": path, "request": read.stored }))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateRequestArgs {
    /// The collection's folder or name.
    pub collection: String,
    /// The folder to create it in; "" (the default) for the collection's top level.
    #[serde(default)]
    pub folder: String,
    /// The request, in the shape get_request returns; fields left out take defaults. E.g.
    /// {"name": "List users", "request": {"method": "GET", "url": "{{base}}/users",
    /// "headers": [{"name": "Accept", "value": "application/json"}]}}. Bodies:
    /// {"type": "text", "kind": "json", "text": "..."}, {"type": "form", "fields": [...]}.
    pub request: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateRequestArgs {
    pub collection: String,
    /// The request file, as get_tree gives it.
    pub path: String,
    /// The fields to change, in the shape get_request returns; fields left out keep their
    /// value, and inside `request` the same holds for method, url, headers and body.
    pub request: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NameArgs {
    pub collection: String,
    /// For create_folder the parent folder ("" for the top level); for rename the request or
    /// folder.
    #[serde(default)]
    pub path: String,
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveArgs {
    pub collection: String,
    /// The request or folder to move.
    pub path: String,
    /// Where to: a folder, or "" for the top level. It goes last there.
    pub folder: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OrderArgs {
    pub collection: String,
    pub path: String,
    /// Position among its siblings; lower first. Siblings use steps of 10.
    pub order: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HeadersArgs {
    pub collection: String,
    /// A folder, or "" for the collection.
    #[serde(default)]
    pub path: String,
    /// Replaces the headers its requests inherit. A header named here replaces inherited ones
    /// of the same name; a disabled one turns them off.
    pub headers: Vec<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthArgs {
    pub collection: String,
    /// A folder, or "" for the collection.
    #[serde(default)]
    pub path: String,
    /// E.g. {"type": "bearer", "token": "{{token}}"}, {"type": "basic", "username": "...",
    /// "password": "{{password}}"}, {"type": "inherit"}, {"type": "none"}. Use variables for
    /// credentials: literal secret values are refused.
    pub auth: Value,
}

fn parsed<T: serde::de::DeserializeOwned>(value: Value, what: &str) -> Result<T, McpError> {
    serde_json::from_value(value).map_err(|e| McpError::InvalidArgument(format!("Not {what}: {e}")))
}

impl Server {
    fn create_request_in(&self, args: CreateRequestArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        let stored = overlay(blank(), args.request)?;
        guarded(self, &root, &stored)?;
        let path = collection::create_request(&root, &args.folder, &stored)?;
        saved(&root, &path)
    }

    fn update_request_in(&self, args: UpdateRequestArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        let current = collection::read_request(&root, &args.path)?;
        let base = serde_json::to_value(&current.stored)
            .map_err(|e| McpError::Engine(wbl_engine::EngineError::internal(e.to_string())))?;
        let stored = overlay(base, args.request)?;
        guarded(self, &root, &stored)?;
        collection::save_request(&root, &args.path, &stored, &current.hash)?;
        saved(&root, &args.path)
    }

    fn rename_in(&self, args: NameArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        if !args.path.is_empty() && root.join(&args.path).is_dir() {
            collection::rename_folder(&root, &args.path, &args.name)?;
            return Ok(json!({ "path": args.path }));
        }
        Ok(json!({ "path": collection::rename_request(&root, &args.path, &args.name)? }))
    }

    fn headers_in(&self, args: HeadersArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        let list = args
            .headers
            .into_iter()
            .map(|mut h| {
                if let Some(o) = h.as_object_mut() {
                    o.entry("enabled").or_insert(Value::Bool(true));
                }
                parsed::<wbl_core::Header>(h, "a header")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let text = serde_json::to_string(&list).unwrap_or_default();
        wbl_engine::resolve::guard(&root, &text, &self.state.secrets)?;
        headers::save_headers(&root, &args.path, &list)?;
        Ok(json!({ "headers": headers::headers_of(&root, &args.path)? }))
    }

    fn auth_in(&self, args: AuthArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        let text = args.auth.to_string();
        let value: wbl_core::auth::Auth = parsed(args.auth, "an auth")?;
        wbl_engine::resolve::guard(&root, &text, &self.state.secrets)?;
        auth::save_auth(&root, &args.path, &value)?;
        Ok(json!({ "auth": auth::auth_of(&root, &args.path)? }))
    }
}

#[tool_router(router = request_tools, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "Creates a request file in a collection (in a folder, last among its \
siblings) and returns its path and canonical form."
    )]
    async fn create_request(
        &self,
        Parameters(args): Parameters<CreateRequestArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.create_request_in(args)))
    }

    #[tool(
        description = "Changes a request file: the fields given replace the current ones, \
the rest are kept. Returns the canonical form. Use {{variables}} for credentials; a literal \
secret value is refused."
    )]
    async fn update_request(
        &self,
        Parameters(args): Parameters<UpdateRequestArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.update_request_in(args)))
    }

    #[tool(
        description = "Creates a folder (`path` is its parent, \"\" for the top level) and \
returns its path."
    )]
    async fn create_folder(
        &self,
        Parameters(args): Parameters<NameArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|root| {
            Ok(json!({ "path": collection::create_folder(&root, &args.path, &args.name)? }))
        })))
    }

    #[tool(
        description = "Renames a request (its file follows; returns the new path) or a \
folder (its path stays)."
    )]
    async fn rename(
        &self,
        Parameters(args): Parameters<NameArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.rename_in(args)))
    }

    #[tool(
        name = "move",
        description = "Moves a request or a folder (with everything in it) into \
another folder of the same collection, last among its new siblings. Returns the new path."
    )]
    async fn move_node(
        &self,
        Parameters(args): Parameters<MoveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|root| {
            Ok(json!({ "path": collection::move_node(&root, &args.path, &args.folder)? }))
        })))
    }

    #[tool(description = "Sets the position of a request or folder among its siblings.")]
    async fn reorder(
        &self,
        Parameters(args): Parameters<OrderArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|root| {
            collection::set_order(&root, &args.path, args.order)?;
            Ok(json!({ "path": args.path, "order": args.order }))
        })))
    }

    #[tool(description = "Copies a request file next to it and returns the copy's path.")]
    async fn duplicate(
        &self,
        Parameters(args): Parameters<crate::tools::read::NodeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|root| {
            Ok(json!({ "path": collection::duplicate_request(&root, &args.path)? }))
        })))
    }

    #[tool(
        description = "Deletes a request file, or a folder with everything in it. The \
collection is in git, so the user can restore it there."
    )]
    async fn delete(
        &self,
        Parameters(args): Parameters<crate::tools::read::NodeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.root(&args.collection).and_then(|root| {
            collection::delete(&root, &args.path)?;
            Ok(json!({ "deleted": args.path }))
        })))
    }

    #[tool(
        description = "Sets the headers a folder (or the collection, path \"\") adds to every \
request in it."
    )]
    async fn set_headers(
        &self,
        Parameters(args): Parameters<HeadersArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.headers_in(args)))
    }

    #[tool(
        description = "Sets the auth a folder (or the collection, path \"\") gives the requests \
in it that inherit."
    )]
    async fn set_auth(
        &self,
        Parameters(args): Parameters<AuthArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.auth_in(args)))
    }
}

#[cfg(test)]
pub(crate) mod tests;
