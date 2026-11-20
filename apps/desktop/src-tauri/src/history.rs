//! Recording sends (FR-8) with every secret value masked (D-H12), and the history commands.

use tauri::State;
use wbl_core::interpolate::mask;
use wbl_http::{Response, ResponseBody, ResponseHeader};
use wbl_store::{HistoryEntry, HistorySummary, NewEntry, Outcome};

use crate::error::AppError;
use crate::ipc::{HistoryIdArgs, HistoryListArgs};
use crate::logging::logged;
use crate::state::AppState;

/// Body bytes kept per entry (FR-8.2).
const KEPT_BODY: usize = 256 * 1024;

fn cut(text: &str) -> (String, bool) {
    if text.len() <= KEPT_BODY {
        return (text.to_owned(), false);
    }
    let mut end = KEPT_BODY;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), true)
}

/// The response as kept: secrets masked in headers and body, the body cut to 256 KB, binary
/// bodies and spill files dropped (the files are gone after a restart).
pub fn masked_response(response: &Response, secrets: &[String]) -> Response {
    let (text, cut_short) = match &response.body.text {
        Some(text) => {
            let (kept, cut_short) = cut(&mask(text, secrets));
            (Some(kept), cut_short)
        }
        None => (None, false),
    };
    Response {
        headers: response
            .headers
            .iter()
            .map(|h| ResponseHeader {
                name: h.name.clone(),
                value: mask(&h.value, secrets),
            })
            .collect(),
        body: ResponseBody {
            text,
            base64: None,
            truncated: response.body.truncated || cut_short || response.body.base64.is_some(),
            file: None,
            ..response.body.clone()
        },
        url: mask(&response.url, secrets),
        ..response.clone()
    }
}

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
    use wbl_http::Timing;

    use super::*;

    fn response(text: Option<String>, base64: Option<String>) -> Response {
        Response {
            status: 200,
            reason: Some("OK".into()),
            http_version: "HTTP/1.1".into(),
            url: "https://x/?key=s3cret".into(),
            headers: vec![ResponseHeader {
                name: "x-echo".into(),
                value: "Bearer s3cret".into(),
            }],
            body: ResponseBody {
                text,
                base64,
                size: 10,
                truncated: false,
                file: Some("/tmp/body".into()),
                content_type: None,
            },
            timing: Timing {
                headers_ms: 1,
                total_ms: 2,
                dns_ms: None,
                connect_ms: Some(1),
            },
            stream_events: 0,
        }
    }

    #[test]
    fn kept_responses_have_secrets_masked_and_long_bodies_cut() {
        let secrets = vec!["s3cret".to_owned()];
        let long = format!("s3cret {}é", "x".repeat(KEPT_BODY));

        let kept = masked_response(&response(Some(long), None), &secrets);
        let binary = masked_response(&response(None, Some("AAAA".into())), &secrets);

        assert_eq!(kept.url, "https://x/?key=••••••");
        assert_eq!(kept.headers[0].value, "Bearer ••••••");
        let text = kept.body.text.unwrap();
        assert!(text.starts_with("••••••"));
        assert!(text.len() <= KEPT_BODY);
        assert!(kept.body.truncated);
        assert_eq!(kept.body.file, None, "spill files are gone after a restart");
        assert_eq!(binary.body.base64, None);
        assert!(binary.body.truncated);
    }

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
