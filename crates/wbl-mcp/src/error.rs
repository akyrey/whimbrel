//! `McpError`: what a tool reports when it cannot do what it was asked. The model sees the
//! stable code and a message without URLs, header values or secrets (D-H11).

use rmcp::model::CallToolResult;
use serde_json::json;
use wbl_engine::EngineError;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum McpError {
    /// The collection was not named with `--collection` when the server started.
    #[error("{0}")]
    NotReachable(String),
    /// Sending with a protected environment needs the app (D-H23).
    #[error("The environment {0} is protected: send with it from the app")]
    ProtectedEnvironment(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    /// From the engine or the file format, with its own code.
    #[error("{}", .0.message)]
    Engine(EngineError),
}

impl McpError {
    pub fn code(&self) -> &str {
        match self {
            Self::NotReachable(_) => "not_reachable",
            Self::ProtectedEnvironment(_) => "protected_environment",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::NotFound(_) => "not_found",
            Self::Engine(e) => &e.code,
        }
    }

    /// The error as a tool result the model reads (`isError`), with the code.
    pub fn into_result(self) -> CallToolResult {
        let mut body = json!({ "code": self.code(), "message": self.to_string() });
        if let Self::Engine(EngineError {
            detail: Some(detail),
            ..
        }) = &self
        {
            body["detail"] = detail.clone();
        }
        CallToolResult::structured_error(body)
    }
}

impl From<EngineError> for McpError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

impl From<wbl_format::FormatError> for McpError {
    fn from(error: wbl_format::FormatError) -> Self {
        Self::Engine(error.into())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn results_carry_the_code_message_and_detail() {
        let engine =
            EngineError::new("unresolved", "Set host").with_detail(json!({"undefined": ["host"]}));

        let result = McpError::from(engine).into_result();
        let plain =
            McpError::NotReachable("Not one of the server's collections".into()).into_result();

        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            result.structured_content.unwrap(),
            json!({"code": "unresolved", "message": "Set host", "detail": {"undefined": ["host"]}})
        );
        assert_eq!(
            plain.structured_content.unwrap()["code"],
            json!("not_reachable")
        );
    }

    #[test]
    fn format_errors_keep_the_engine_codes() {
        let error = McpError::from(wbl_format::FormatError::Path("../x".into()));
        assert_eq!(error.code(), "invalid_path");
    }
}
