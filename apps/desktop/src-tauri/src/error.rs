//! `AppError`: the only error shape the frontend sees. A stable code, a message safe to show,
//! and for invalid requests the field at fault.

use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AppError {
    /// Stable `snake_case` code.
    pub code: String,
    /// Safe to show to the user.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown", optional)]
    pub detail: Option<serde_json::Value>,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }
}

impl From<wbl_http::HttpError> for AppError {
    fn from(error: wbl_http::HttpError) -> Self {
        let detail = error.field.as_ref().map(|field| json!({ "field": field }));
        Self {
            detail,
            ..Self::new(error.code.as_str(), error.message)
        }
    }
}

impl From<roost_shell::ShellError> for AppError {
    fn from(error: roost_shell::ShellError) -> Self {
        Self::new(error.code, error.message)
    }
}

#[cfg(test)]
mod tests {
    use wbl_http::{HttpError, HttpErrorCode};

    use super::*;

    #[test]
    fn http_errors_keep_their_code_and_field() {
        let error = AppError::from(HttpError::invalid("url", "Enter a URL"));

        assert_eq!(error.code, "invalid_request");
        assert_eq!(error.message, "Enter a URL");
        assert_eq!(error.detail, Some(json!({ "field": "url" })));
        assert_eq!(
            AppError::from(HttpError::new(HttpErrorCode::Timeout, "late")).detail,
            None
        );
    }

    #[test]
    fn serialises_without_an_empty_detail() {
        let json = serde_json::to_value(AppError::internal("x")).ok();
        assert_eq!(json, Some(json!({ "code": "internal", "message": "x" })));
    }
}
