//! The runner (FR-6.3): a list of steps (requests of a folder, in tree order) sent one after the
//! other with the request's own checks; each result is streamed as it comes.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;
use wbl_core::example::Example;
use wbl_format::collection;

use crate::checks::Checks;
use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::RequestSendArgs;
use crate::logging::logged;
use crate::secrets::Secrets;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize, TS)]
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

#[derive(Debug, Clone, Deserialize, TS)]
pub struct RunStep {
    /// The request file, relative to the collection.
    pub path: String,
    pub mode: StepMode,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RunStartArgs {
    pub run_id: String,
    pub root: String,
    #[ts(optional)]
    pub environment: Option<String>,
    pub steps: Vec<RunStep>,
    pub stop_on_failure: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RunCancelArgs {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    /// Position in the run, from 0 (a step run per example gives several).
    #[ts(type = "number")]
    pub index: usize,
    pub path: String,
    pub name: String,
    pub example: Option<String>,
    pub method: String,
    pub status: Option<u16>,
    #[ts(type = "number | null")]
    pub total_ms: Option<u64>,
    pub passed: bool,
    pub error: Option<String>,
    pub checks: Option<Checks>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    #[ts(type = "number")]
    pub passed: usize,
    #[ts(type = "number")]
    pub failed: usize,
    /// Not run: after a failure with "stop on failure", or after cancelling.
    #[ts(type = "number")]
    pub skipped: usize,
    pub cancelled: bool,
}

/// Runs in progress, by id.
#[derive(Default)]
pub struct Runs(Mutex<HashMap<String, CancellationToken>>);

fn poisoned() -> AppError {
    AppError::internal("The runner is unusable; restart the app")
}

impl Runs {
    fn start(&self, id: &str) -> Result<CancellationToken, AppError> {
        let token = CancellationToken::new();
        self.0
            .lock()
            .map_err(|_| poisoned())?
            .insert(id.to_owned(), token.clone());
        Ok(token)
    }

    fn finish(&self, id: &str) -> Result<(), AppError> {
        self.0.lock().map_err(|_| poisoned())?.remove(id);
        Ok(())
    }

    fn cancel(&self, id: &str) -> Result<bool, AppError> {
        let token = self.0.lock().map_err(|_| poisoned())?.get(id).cloned();
        token.as_ref().inspect(|t| t.cancel());
        Ok(token.is_some())
    }
}

/// One send of a run: the request file and the example it uses.
struct Planned {
    path: String,
    stored: wbl_format::StoredRequest,
    example: Option<Example>,
}

fn expand(root: &std::path::Path, steps: &[RunStep]) -> Result<Vec<Planned>, AppError> {
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
                        AppError::new("not_found", format!("{} has no example {id}", stored.name))
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

fn send_args(args: &RunStartArgs, planned: &Planned) -> RequestSendArgs {
    let s = &planned.stored;
    RequestSendArgs {
        request_id: format!("run:{}", args.run_id),
        request: s.request.clone(),
        root: Some(args.root.clone()),
        environment: args.environment.clone(),
        example: planned.example.clone(),
        settings: Some(s.settings),
        name: Some(s.name.clone()),
        path: Some(planned.path.clone()),
        auth: Some(s.auth.clone()),
        variables: Some(s.variables.clone()),
        captures: Some(s.captures.clone()),
        assertions: Some(s.assertions.clone()),
    }
}

async fn step(
    state: &AppState,
    collections: &Collections,
    secrets: &Secrets,
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
    match crate::commands::send(
        state,
        collections,
        secrets,
        &mut |_| {},
        send_args(args, planned),
    )
    .await
    {
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

/// Runs `args.steps`; `on_step` gets each result as it is known.
pub async fn run(
    state: &AppState,
    runs: &Runs,
    collections: &Collections,
    secrets: &Secrets,
    args: &RunStartArgs,
    on_step: &mut (dyn FnMut(StepResult) + Send),
) -> Result<RunSummary, AppError> {
    let root = collections.opened(&args.root)?;
    let planned = expand(&root, &args.steps)?;
    let token = runs.start(&args.run_id)?;
    let mut summary = RunSummary::default();
    for (index, p) in planned.iter().enumerate() {
        if token.is_cancelled() {
            summary.cancelled = true;
            summary.skipped = planned.len() - index;
            break;
        }
        let result = step(state, collections, secrets, args, index, p).await;
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
    runs.finish(&args.run_id)?;
    Ok(summary)
}

/// Stops a run: the request in flight is cancelled and nothing after it is sent.
pub fn cancel(state: &AppState, runs: &Runs, args: &RunCancelArgs) -> Result<bool, AppError> {
    let known = runs.cancel(&args.run_id)?;
    state.in_flight.cancel(&format!("run:{}", args.run_id))?;
    Ok(known)
}

#[tauri::command]
pub async fn run_start(
    state: State<'_, AppState>,
    runs: State<'_, Runs>,
    collections: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RunStartArgs,
    on_step: tauri::ipc::Channel<StepResult>,
) -> Result<RunSummary, AppError> {
    let mut forward = move |result: StepResult| {
        let _ = on_step.send(result);
    };
    logged(
        "run_start",
        run(&state, &runs, &collections, &secrets, &args, &mut forward),
    )
    .await
}

#[tauri::command]
pub async fn run_cancel(
    state: State<'_, AppState>,
    runs: State<'_, Runs>,
    args: RunCancelArgs,
) -> Result<bool, AppError> {
    logged("run_cancel", async { cancel(&state, &runs, &args) }).await
}

#[cfg(test)]
mod tests;
