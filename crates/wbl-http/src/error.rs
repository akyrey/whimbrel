//! Why a request failed, with a stable code for the frontend and a message safe to show: built
//! without the URL, which may hold a token in its query.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum HttpErrorCode {
    /// The request could not be built (bad method, URL or header).
    InvalidRequest,
    /// No connection: DNS, refused, unreachable.
    Connect,
    /// The TLS handshake failed: an untrusted or expired certificate, a wrong host name.
    Tls,
    Timeout,
    /// Too many redirects, or a redirect that could not be followed.
    Redirect,
    /// The response body could not be read or decoded.
    Body,
    Cancelled,
    /// Anything else at the HTTP level.
    Http,
}

impl HttpErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::Connect => "connect",
            Self::Tls => "tls",
            Self::Timeout => "timeout",
            Self::Redirect => "redirect",
            Self::Body => "body",
            Self::Cancelled => "cancelled",
            Self::Http => "http",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct HttpError {
    pub code: HttpErrorCode,
    pub message: String,
    /// For `invalid_request`: the field at fault (`method`, `url`, `headers`).
    pub field: Option<String>,
}

impl HttpError {
    pub fn new(code: HttpErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
        }
    }

    pub fn invalid(field: &str, message: impl Into<String>) -> Self {
        Self {
            field: Some(field.to_owned()),
            ..Self::new(HttpErrorCode::InvalidRequest, message)
        }
    }
}

impl From<wbl_core::RequestError> for HttpError {
    fn from(error: wbl_core::RequestError) -> Self {
        Self::invalid(error.field(), error.to_string())
    }
}

/// The error and its causes, without the URL.
pub(crate) fn describe(error: &reqwest::Error) -> String {
    let mut parts = vec![strip_url(error)];
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        let text = cause.to_string();
        if parts.last() != Some(&text) {
            parts.push(text);
        }
        source = cause.source();
    }
    parts.join(": ")
}

fn strip_url(error: &reqwest::Error) -> String {
    // `without_url` takes the error by value; its Display is the kind alone.
    let kind = if error.is_timeout() {
        "timed out"
    } else if error.is_connect() {
        "connection failed"
    } else if error.is_redirect() {
        "redirect failed"
    } else if error.is_body() || error.is_decode() {
        "the body could not be read"
    } else if error.is_builder() {
        "the request could not be built"
    } else {
        "the request failed"
    };
    kind.to_owned()
}

/// TLS failures reach us as connection errors; their causes name the certificate or handshake.
fn is_tls(description: &str) -> bool {
    let lower = description.to_ascii_lowercase();
    [
        "certificate",
        "tls",
        "handshake",
        "unknownissuer",
        "notvalidforname",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

impl From<reqwest::Error> for HttpError {
    fn from(error: reqwest::Error) -> Self {
        let detail = describe(&error);
        let (code, prefix) = if error.is_timeout() {
            (HttpErrorCode::Timeout, "No response in time")
        } else if error.is_connect() && is_tls(&detail) {
            (HttpErrorCode::Tls, "The secure connection failed")
        } else if error.is_connect() {
            (HttpErrorCode::Connect, "Could not connect")
        } else if error.is_redirect() {
            (
                HttpErrorCode::Redirect,
                "The redirect could not be followed",
            )
        } else if error.is_body() || error.is_decode() {
            (HttpErrorCode::Body, "The response body could not be read")
        } else {
            (HttpErrorCode::Http, "The request failed")
        };
        Self::new(code, format!("{prefix} ({detail})"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable_snake_case() {
        let codes = [
            (HttpErrorCode::InvalidRequest, "invalid_request"),
            (HttpErrorCode::Connect, "connect"),
            (HttpErrorCode::Tls, "tls"),
            (HttpErrorCode::Timeout, "timeout"),
            (HttpErrorCode::Redirect, "redirect"),
            (HttpErrorCode::Body, "body"),
            (HttpErrorCode::Cancelled, "cancelled"),
            (HttpErrorCode::Http, "http"),
        ];
        for (code, text) in codes {
            assert_eq!(code.as_str(), text);
            assert_eq!(
                serde_json::to_value(code).ok(),
                Some(serde_json::json!(text))
            );
        }
    }

    #[test]
    fn certificate_problems_are_recognised_as_tls() {
        assert!(is_tls(
            "connection failed: invalid peer certificate: UnknownIssuer"
        ));
        assert!(is_tls("received fatal alert: HandshakeFailure"));
        assert!(!is_tls(
            "connection failed: Connection refused (os error 61)"
        ));
    }

    #[test]
    fn request_errors_name_their_field() {
        let error = HttpError::from(wbl_core::RequestError::Header("bad".into()));
        assert_eq!(error.field.as_deref(), Some("headers"));
        assert_eq!(error.code, HttpErrorCode::InvalidRequest);
    }
}
