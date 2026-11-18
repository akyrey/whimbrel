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

    pub fn with_detail(self, detail: serde_json::Value) -> Self {
        Self {
            detail: Some(detail),
            ..self
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

impl From<wbl_format::FormatError> for AppError {
    fn from(error: wbl_format::FormatError) -> Self {
        use wbl_format::FormatError as F;
        let code = match &error {
            F::Syntax(_) | F::Missing(_) | F::Invalid { .. } => "invalid_file",
            F::TooNew { .. } => "too_new",
            F::Io(_) => "io",
            F::Conflict => "conflict",
            F::Path(_) => "invalid_path",
        };
        Self::new(code, error.to_string())
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
    fn format_errors_have_stable_codes() {
        use wbl_format::FormatError as F;
        let code = |e: F| AppError::from(e).code;
        assert_eq!(code(F::Syntax("x".into())), "invalid_file");
        assert_eq!(code(F::Missing("name".into())), "invalid_file");
        assert_eq!(
            code(F::TooNew {
                found: 2,
                supported: 1
            }),
            "too_new"
        );
        assert_eq!(code(F::Io("x".into())), "io");
        assert_eq!(code(F::Conflict), "conflict");
        assert_eq!(code(F::Path("x".into())), "invalid_path");
    }

    #[test]
    fn serialises_without_an_empty_detail() {
        let json = serde_json::to_value(AppError::internal("x")).ok();
        assert_eq!(json, Some(json!({ "code": "internal", "message": "x" })));
    }
}
