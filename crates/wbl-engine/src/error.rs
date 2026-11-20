//! `EngineError`: a stable `snake_case` code, a message safe to show, and sometimes a detail
//! (the field at fault, the variables that are missing). The app turns it into its own error
//! unchanged; the MCP server into a tool error.

use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Serialize, thiserror::Error)]
#[error("{code}: {message}")]
pub struct EngineError {
    /// Stable `snake_case` code.
    pub code: String,
    /// Safe to show to the user: never a URL, a header value or a secret.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

impl EngineError {
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

impl From<wbl_http::HttpError> for EngineError {
    fn from(error: wbl_http::HttpError) -> Self {
        let detail = error.field.as_ref().map(|field| json!({ "field": field }));
        Self {
            detail,
            ..Self::new(error.code.as_str(), error.message)
        }
    }
}

impl From<wbl_format::FormatError> for EngineError {
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

impl From<wbl_auth::FlowError> for EngineError {
    fn from(error: wbl_auth::FlowError) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use wbl_http::{HttpError, HttpErrorCode};

    use super::*;

    #[test]
    fn http_errors_keep_their_code_and_field() {
        let error = EngineError::from(HttpError::invalid("url", "Enter a URL"));

        assert_eq!(error.code, "invalid_request");
        assert_eq!(error.message, "Enter a URL");
        assert_eq!(error.detail, Some(json!({ "field": "url" })));
        assert_eq!(
            EngineError::from(HttpError::new(HttpErrorCode::Timeout, "late")).detail,
            None
        );
    }

    #[test]
    fn format_errors_have_stable_codes() {
        use wbl_format::FormatError as F;
        let code = |e: F| EngineError::from(e).code;
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
        let json = serde_json::to_value(EngineError::internal("x")).ok();
        assert_eq!(json, Some(json!({ "code": "internal", "message": "x" })));
    }
}
