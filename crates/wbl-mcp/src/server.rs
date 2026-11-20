//! The MCP server: its state, what it tells clients about itself, and its tools (`tools/`).

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{CallToolResult, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, tool_handler};
use serde::Serialize;
use wbl_engine::secrets::Secrets;

use crate::error::McpError;
use crate::reachable::Collections;

const INSTRUCTIONS: &str = "Whimbrel collections are folders of TOML files in git; this server \
reads and writes them as the Whimbrel app does, and sends requests. Name a collection by its \
folder or name (list_collections). Paths inside a collection are relative, with '/'. Secret \
values never appear: secret variables are listed by name, and known secret values in requests \
and responses are masked. Secrets are set in the Whimbrel app, which also reloads every change \
made here.";

/// What the tools share. Cloning is cheap: everything is behind one `Arc`.
#[derive(Clone)]
pub struct Server {
    pub(crate) state: Arc<State>,
}

pub(crate) struct State {
    pub collections: Collections,
    pub secrets: Secrets,
}

impl Server {
    /// A server for `folders` (each must hold a collection), with secrets from `secrets`.
    pub fn new(folders: &[PathBuf], secrets: Secrets) -> Result<Self, McpError> {
        Ok(Self {
            state: Arc::new(State {
                collections: Collections::open(folders)?,
                secrets,
            }),
        })
    }

    pub(crate) fn root(&self, collection: &str) -> Result<PathBuf, McpError> {
        self.state.collections.root(collection).map(PathBuf::from)
    }

    fn tool_router() -> ToolRouter<Self> {
        Self::read_tools() + Self::request_tools() + Self::environment_tools()
    }
}

/// The tool result for `result`: its value as structured content, or the error with its code.
pub(crate) fn reply<T: Serialize>(result: Result<T, McpError>) -> CallToolResult {
    let value = result.and_then(|value| {
        serde_json::to_value(value)
            .map_err(|e| McpError::Engine(wbl_engine::EngineError::internal(e.to_string())))
    });
    match value {
        Ok(value) => CallToolResult::structured(value),
        Err(error) => error.into_result(),
    }
}

#[tool_handler]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("whimbrel", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}
