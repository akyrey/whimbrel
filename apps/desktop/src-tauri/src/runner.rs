//! The runner (FR-6.3): a list of steps (requests of a folder, in tree order) sent one after the
//! other with the request's own checks; each result is streamed as it comes.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Deserialize;
use tauri::State;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::collections::Collections;
use crate::error::AppError;
use crate::ipc::RequestSendArgs;
use crate::logging::logged;
use crate::secrets::Secrets;
use crate::state::AppState;

pub use wbl_engine::runner::{RunStartArgs, RunStep, RunSummary, StepMode, StepResult};

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RunCancelArgs {
    pub run_id: String,
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

/// Each step goes through the app's `send`: registry (so it can be cancelled) and history.
struct AppDispatch<'a> {
    state: &'a AppState,
    collections: &'a Collections,
    secrets: &'a Secrets,
}

impl wbl_engine::runner::Dispatch for AppDispatch<'_> {
    async fn send(
        &self,
        args: RequestSendArgs,
    ) -> Result<crate::checks::Sent, wbl_engine::EngineError> {
        Ok(crate::commands::send(
            self.state,
            self.collections,
            self.secrets,
            &mut |_| {},
            args,
        )
        .await?)
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
    let token = runs.start(&args.run_id)?;
    let dispatch = AppDispatch {
        state,
        collections,
        secrets,
    };
    let result = wbl_engine::runner::run(&dispatch, &root, args, &token, on_step).await;
    runs.finish(&args.run_id)?;
    Ok(result?)
}

/// Stops a run: the request in flight is cancelled and nothing after it is sent.
pub fn cancel(state: &AppState, runs: &Runs, args: &RunCancelArgs) -> Result<bool, AppError> {
    let known = runs.cancel(&args.run_id)?;
    state
        .in_flight
        .cancel(&wbl_engine::runner::request_id(&args.run_id))?;
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
