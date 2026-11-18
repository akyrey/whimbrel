//! A request as the user wrote it: method, URL, headers and body, before variables are resolved
//! (Phase 3) and before it is sent (`wbl-http`).

use serde::{Deserialize, Serialize};

/// A request header. Order and duplicates are kept: both matter to some servers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Header {
    pub name: String,
    pub value: String,
    /// Disabled headers stay in the request but are not sent.
    pub enabled: bool,
}

/// How a text body is highlighted, and the `Content-Type` it gets when none is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BodyKind {
    Json,
    Xml,
    Text,
}

impl BodyKind {
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Xml => "application/xml",
            Self::Text => "text/plain; charset=utf-8",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Body {
    None,
    Text { kind: BodyKind, text: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Request {
    /// Any RFC 9110 token: `GET`, `QUERY`, `PROPFIND`, custom methods.
    pub method: String,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Body,
}

/// Why a request cannot be sent, with a message for the field at fault.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RequestError {
    #[error("{0}")]
    Method(String),
    #[error("{0}")]
    Url(String),
    #[error("{0}")]
    Header(String),
}

impl RequestError {
    /// The field the message is about, for the frontend to point at.
    pub fn field(&self) -> &'static str {
        match self {
            Self::Method(_) => "method",
            Self::Url(_) => "url",
            Self::Header(_) => "headers",
        }
    }
}

/// `tchar` from RFC 9110 §5.6.2: the characters of methods and header names.
fn is_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

impl Request {
    /// Checks everything that would stop the request from being sent, the first problem first.
    pub fn validate(&self) -> Result<(), RequestError> {
        self.validate_method()?;
        self.validate_url()?;
        self.enabled_headers().try_for_each(validate_header)
    }

    fn validate_method(&self) -> Result<(), RequestError> {
        if self.method.is_empty() {
            return Err(RequestError::Method("The method is empty".into()));
        }
        if !is_token(&self.method) {
            return Err(RequestError::Method(format!(
                "The method {:?} is not a valid HTTP method",
                self.method
            )));
        }
        Ok(())
    }

    fn validate_url(&self) -> Result<(), RequestError> {
        let url = self.url.trim();
        if url.is_empty() {
            return Err(RequestError::Url("Enter a URL".into()));
        }
        let lower = url.to_ascii_lowercase();
        if !(lower.starts_with("http://") || lower.starts_with("https://")) {
            return Err(RequestError::Url(
                "The URL needs http:// or https://".into(),
            ));
        }
        url::Url::parse(url)
            .map(|_| ())
            .map_err(|e| RequestError::Url(format!("The URL is not valid: {e}")))
    }

    /// The headers that are sent, in order.
    pub fn enabled_headers(&self) -> impl Iterator<Item = &Header> {
        self.headers.iter().filter(|h| h.enabled)
    }

    /// The `Content-Type` to add for the body, when the user has not set one.
    pub fn default_content_type(&self) -> Option<&'static str> {
        let Body::Text { kind, .. } = &self.body else {
            return None;
        };
        let has_one = self
            .enabled_headers()
            .any(|h| h.name.eq_ignore_ascii_case("content-type"));
        (!has_one).then(|| kind.content_type())
    }
}

fn validate_header(header: &Header) -> Result<(), RequestError> {
    if header.name.is_empty() {
        return Err(RequestError::Header("A header has no name".into()));
    }
    if !is_token(&header.name) {
        return Err(RequestError::Header(format!(
            "The header name {:?} is not valid",
            header.name
        )));
    }
    if header.value.contains(['\r', '\n']) {
        return Err(RequestError::Header(format!(
            "The value of {} has a line break",
            header.name
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
