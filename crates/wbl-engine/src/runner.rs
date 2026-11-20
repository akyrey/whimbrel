//! The runner (FR-6.3): a list of steps (requests of a folder, in tree order) sent one after the
//! other with the request's own checks; each result is streamed as it comes.

use std::future::Future;

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use wbl_core::example::Example;
use wbl_format::collection::{self, Node};

use crate::args::RequestSendArgs;
use crate::checks::{Checks, Sent};
use crate::error::EngineError;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum StepMode {
    /// The request as it is.
    Base,
    /// With one saved example.
    Example { id: String },
    /// Once per saved example (once as it is when it has none).
    EachExample,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RunStep {
    /// The request file, relative to the collection.
    pub path: String,
    pub mode: StepMode,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RunStartArgs {
    pub run_id: String,
    pub root: String,
    #[cfg_attr(feature = "ts", ts(optional))]
    pub environment: Option<String>,
    pub steps: Vec<RunStep>,
    pub stop_on_failure: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    /// Position in the run, from 0 (a step run per example gives several).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub index: usize,
    pub path: String,
    pub name: String,
    pub example: Option<String>,
    pub method: String,
    pub status: Option<u16>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub total_ms: Option<u64>,
    pub passed: bool,
    pub error: Option<String>,
    pub checks: Option<Checks>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub passed: usize,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub failed: usize,
    /// Not run: after a failure with "stop on failure", or after cancelling.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub skipped: usize,
    pub cancelled: bool,
}

/// Sends one step of a run. The app sends through its registry and history; the MCP server
/// sends directly.
pub trait Dispatch {
    fn send(&self, args: RequestSendArgs)
    -> impl Future<Output = Result<Sent, EngineError>> + Send;
}

/// One send of a run: the request file and the example it uses.
struct Planned {
    path: String,
    stored: wbl_format::StoredRequest,
    example: Option<Example>,
}

fn expand(root: &std::path::Path, steps: &[RunStep]) -> Result<Vec<Planned>, EngineError> {
    let mut out = Vec::new();
    for step in steps {
        let stored = collection::read_request(root, &step.path)?.stored;
        let examples: Vec<Option<Example>> = match &step.mode {
            StepMode::Base => vec![None],
            StepMode::Example { id } => vec![Some(
                stored
                    .examples
                    .iter()
                    .find(|e| e.id == *id)
                    .cloned()
                    .ok_or_else(|| {
                        EngineError::new(
                            "not_found",
                            format!("{} has no example {id}", stored.name),
                        )
                    })?,
            )],
            StepMode::EachExample if stored.examples.is_empty() => vec![None],
            StepMode::EachExample => stored.examples.iter().cloned().map(Some).collect(),
        };
        out.extend(examples.into_iter().map(|example| Planned {
            path: step.path.clone(),
            stored: stored.clone(),
            example,
        }));
    }
    Ok(out)
}

fn requests_under(nodes: &[Node], folder: &str, inside: bool, out: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Request(r) if inside => out.push(r.path.clone()),
            Node::Folder(f) => requests_under(&f.children, folder, inside || f.path == folder, out),
            _ => {}
        }
    }
}

/// The request files in `folder` (`""`: the whole collection) and its folders, in tree order.
pub fn requests_in(root: &std::path::Path, folder: &str) -> Result<Vec<String>, EngineError> {
    let tree = collection::load(root)?;
    let mut paths = Vec::new();
    requests_under(&tree.children, folder, folder.is_empty(), &mut paths);
    Ok(paths)
}

/// The request id every send of run `run_id` uses, so cancelling the run cancels its send.
pub fn request_id(run_id: &str) -> String {
    format!("run:{run_id}")
}

fn send_args(args: &RunStartArgs, planned: &Planned) -> RequestSendArgs {
    RequestSendArgs {
        environment: args.environment.clone(),
        example: planned.example.clone(),
        ..RequestSendArgs::stored(
            &request_id(&args.run_id),
            &args.root,
            &planned.path,
            &planned.stored,
        )
    }
}

async fn step<D: Dispatch>(
    dispatch: &D,
    args: &RunStartArgs,
    index: usize,
    planned: &Planned,
) -> StepResult {
    let base = StepResult {
        index,
        path: planned.path.clone(),
        name: planned.stored.name.clone(),
        example: planned.example.as_ref().map(|e| e.name.clone()),
        method: planned.stored.request.method.clone(),
        status: None,
        total_ms: None,
        passed: false,
        error: None,
        checks: None,
    };
    match dispatch.send(send_args(args, planned)).await {
        Ok(sent) => StepResult {
            status: Some(sent.response.status),
            total_ms: Some(sent.response.timing.total_ms),
            passed: sent.checks.passed(),
            checks: Some(sent.checks),
            ..base
        },
        Err(e) => StepResult {
            error: Some(e.message),
            ..base
        },
    }
}

/// Runs `args.steps` of the collection at `root` (already checked by the caller); `on_step`
/// gets each result as it is known. Cancelling `token` stops the run after the current step.
pub async fn run<D: Dispatch>(
    dispatch: &D,
    root: &std::path::Path,
    args: &RunStartArgs,
    token: &CancellationToken,
    on_step: &mut (dyn FnMut(StepResult) + Send),
) -> Result<RunSummary, EngineError> {
    let planned = expand(root, &args.steps)?;
    let mut summary = RunSummary::default();
    for (index, p) in planned.iter().enumerate() {
        if token.is_cancelled() {
            summary.cancelled = true;
            summary.skipped = planned.len() - index;
            break;
        }
        let result = step(dispatch, args, index, p).await;
        let passed = result.passed;
        on_step(result);
        if passed {
            summary.passed += 1;
            continue;
        }
        summary.failed += 1;
        if token.is_cancelled() {
            summary.cancelled = true;
            summary.skipped = planned.len() - index - 1;
            break;
        }
        if args.stop_on_failure {
            summary.skipped = planned.len() - index - 1;
            break;
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests;
