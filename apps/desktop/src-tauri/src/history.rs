//! Recording sends (FR-8) with every secret value masked (D-H12), and the history commands.

use tauri::State;
use wbl_core::interpolate::mask;
use wbl_http::Response;
use wbl_store::{HistoryEntry, HistorySummary, NewEntry, Outcome};

use crate::error::AppError;
use crate::ipc::{HistoryIdArgs, HistoryListArgs};
use crate::logging::logged;
use crate::state::AppState;

pub use wbl_engine::mask::{KEPT_BODY, masked_response};

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Records a send; a history failure is logged by code and never fails the send.
pub fn record(state: &AppState, entry: &NewEntry) {
    if let Some(history) = &state.history
        && history.record(entry).is_err()
    {
        tracing::warn!(code = "history_failed", "a send was not recorded");
    }
}

pub fn outcome(result: &Result<Response, AppError>, secrets: &[String]) -> Outcome {
    match result {
        Ok(response) => Outcome::Completed {
            response: Box::new(masked_response(response, secrets)),
        },
        Err(error) => Outcome::Failed {
            code: error.code.clone(),
            message: mask(&error.message, secrets),
        },
    }
}

fn store(state: &AppState) -> Result<&wbl_store::History, AppError> {
    state
        .history
        .as_ref()
        .ok_or_else(|| AppError::internal("The history is not available"))
}

fn failed(error: wbl_store::StoreError) -> AppError {
    AppError::new("history", error.to_string())
}

#[tauri::command]
pub async fn history_list(
    state: State<'_, AppState>,
    args: HistoryListArgs,
) -> Result<Vec<HistorySummary>, AppError> {
    logged("history_list", async {
        store(&state)?
            .list(&args.query, args.limit, args.offset)
            .map_err(failed)
    })
    .await
}

#[tauri::command]
pub async fn history_get(
    state: State<'_, AppState>,
    args: HistoryIdArgs,
) -> Result<HistoryEntry, AppError> {
    logged("history_get", async {
        store(&state)?
            .get(args.id)
            .map_err(failed)?
            .ok_or_else(|| AppError::new("not_found", "That history entry is gone"))
    })
    .await
}

#[tauri::command]
pub async fn history_clear(state: State<'_, AppState>) -> Result<(), AppError> {
    logged("history_clear", async {
        store(&state)?.clear().map_err(failed)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_are_kept_with_their_message_masked() {
        let error: Result<Response, AppError> =
            Err(AppError::new("connect", "no route to s3cret.example"));
        assert_eq!(
            outcome(&error, &["s3cret".to_owned()]),
            Outcome::Failed {
                code: "connect".into(),
                message: "no route to ••••••.example".into()
            }
        );
    }
}
