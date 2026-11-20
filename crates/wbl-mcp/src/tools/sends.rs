//! Sending a stored request and running a folder (task 12.8), through the engine as the app
//! does, with everything returned masked (`wbl_engine::deliver`). Protected environments are
//! refused: their confirmation lives in the app (D-H23).

use std::path::Path;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use wbl_engine::EngineError;
use wbl_engine::RequestSendArgs;
use wbl_engine::checks::Sent;
use wbl_engine::deliver::send_once;
use wbl_engine::runner::{Dispatch, RunStartArgs, RunStep, StepMode, requests_in};
use wbl_format::collection::{self, environments};

use crate::error::McpError;
use crate::server::{Server, reply};

/// The request id of every send from this server; nothing cancels it by id.
const REQUEST_ID: &str = "mcp";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendArgs {
    pub collection: String,
    /// The request file, as get_tree gives it.
    pub path: String,
    /// The environment's id (list_environments); none when left out.
    pub environment: Option<String>,
    /// The id of one of the request's saved examples to send instead.
    pub example: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunArgs {
    pub collection: String,
    /// The folder to run; "" (the default) for the whole collection.
    #[serde(default)]
    pub folder: String,
    /// The environment's id (list_environments); none when left out.
    pub environment: Option<String>,
    /// Send each request once per saved example (once as it is when it has none).
    #[serde(default)]
    pub each_example: bool,
    /// Stop at the first request that fails or whose assertions fail.
    #[serde(default)]
    pub stop_on_failure: bool,
}

/// Refuses a protected environment; an unknown one fails as the read does.
fn allowed(root: &Path, environment: Option<&str>) -> Result<(), McpError> {
    let Some(id) = environment else {
        return Ok(());
    };
    let read = environments::read(root, id)?;
    if read.environment.protected {
        return Err(McpError::ProtectedEnvironment(read.environment.name));
    }
    Ok(())
}

/// Each step goes out through `send_once`, so a run returns only masked values too.
struct Direct<'a> {
    server: &'a Server,
    root: &'a Path,
}

impl Dispatch for Direct<'_> {
    async fn send(&self, args: RequestSendArgs) -> Result<Sent, EngineError> {
        let sources = self.server.sources(self.root);
        let delivered = send_once(
            self.server.clients(),
            &sources,
            &args,
            &CancellationToken::new(),
        )
        .await?;
        Ok(Sent {
            response: delivered.response,
            checks: delivered.checks,
        })
    }
}

impl Server {
    async fn send_in(&self, args: SendArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        allowed(&root, args.environment.as_deref())?;
        let stored = collection::read_request(&root, &args.path)?.stored;
        let example = match &args.example {
            None => None,
            Some(id) => Some(
                stored
                    .examples
                    .iter()
                    .find(|e| e.id == *id)
                    .cloned()
                    .ok_or_else(|| {
                        McpError::NotFound(format!("{} has no example {id}", stored.name))
                    })?,
            ),
        };
        let send = RequestSendArgs {
            environment: args.environment.clone(),
            example,
            ..RequestSendArgs::stored(REQUEST_ID, &root.to_string_lossy(), &args.path, &stored)
        };
        let sources = self.sources(&root);
        let delivered =
            send_once(self.clients(), &sources, &send, &CancellationToken::new()).await?;
        Ok(serde_json::to_value(delivered).unwrap_or_default())
    }

    async fn run_in(&self, args: RunArgs) -> Result<Value, McpError> {
        let root = self.root(&args.collection)?;
        allowed(&root, args.environment.as_deref())?;
        let mode = if args.each_example {
            StepMode::EachExample
        } else {
            StepMode::Base
        };
        let steps = requests_in(&root, &args.folder)?
            .into_iter()
            .map(|path| RunStep {
                path,
                mode: mode.clone(),
            })
            .collect();
        let start = RunStartArgs {
            run_id: REQUEST_ID.to_owned(),
            root: root.to_string_lossy().into_owned(),
            environment: args.environment,
            steps,
            stop_on_failure: args.stop_on_failure,
        };
        let mut results = Vec::new();
        let dispatch = Direct {
            server: self,
            root: &root,
        };
        let summary = wbl_engine::runner::run(
            &dispatch,
            &root,
            &start,
            &CancellationToken::new(),
            &mut |step| results.push(step),
        )
        .await?;
        Ok(json!({ "summary": summary, "steps": results }))
    }
}

#[tool_router(router = send_tools, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "Sends a stored request with an environment (and optionally one of its \
saved examples), runs its captures and assertions, and returns the request as sent and the \
response. Every known secret value is masked; bodies are cut at 256 kB and binary bodies are \
described, not returned. Protected environments are refused: send with them from the app. An \
OAuth 2 request needs a token got in the app."
    )]
    async fn send_request(
        &self,
        Parameters(args): Parameters<SendArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.send_in(args).await))
    }

    #[tool(
        description = "Runs every request of a folder (or the collection) in tree order with \
an environment, as the app's runner does: captures feed later requests, assertions decide pass \
or fail. Returns a summary and one row per send (status, time, checks, error). Protected \
environments are refused."
    )]
    async fn run_folder(
        &self,
        Parameters(args): Parameters<RunArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(reply(self.run_in(args).await))
    }
}

#[cfg(test)]
mod tests;
