//! What came back, ready for the frontend.

use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResponseHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    /// The shown bytes as text, when they are UTF-8 (a character cut at the end is dropped).
    pub text: Option<String>,
    /// The shown bytes, base64, when they are not text (hex view, image preview).
    pub base64: Option<String>,
    /// Bytes of the body (after decompression), as far as it was read.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    /// What is shown is only the start of the body.
    pub truncated: bool,
    /// The whole body, when it was too long to keep in memory.
    pub file: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    /// From sending to the response headers.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub headers_ms: u64,
    /// From sending to the end of the body.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub total_ms: u64,
    /// Resolving the host name, when the send opened a connection to a name (D-H20).
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub dns_ms: Option<u64>,
    /// TCP and TLS, when the send opened a connection; `None`: it reused one.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub connect_ms: Option<u64>,
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

impl Timing {
    pub(crate) fn of(headers: Duration, total: Duration, phases: crate::phases::Phases) -> Self {
        Self {
            headers_ms: millis(headers),
            total_ms: millis(total),
            dns_ms: phases.dns.map(millis),
            connect_ms: phases.connect_only().map(millis),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub status: u16,
    /// The standard reason for the status, e.g. `Not Found`.
    pub reason: Option<String>,
    /// E.g. `HTTP/1.1`, `HTTP/2.0`.
    pub http_version: String,
    /// Where the request ended, after redirects.
    pub url: String,
    pub headers: Vec<ResponseHeader>,
    pub body: ResponseBody,
    pub timing: Timing,
    /// Events of a stream (SSE, JSON Lines) received; 0 for other bodies.
    pub stream_events: u32,
}

/// The parts of a response known before its body is read.
pub(crate) struct Head {
    status: u16,
    reason: Option<String>,
    http_version: String,
    url: String,
    headers: Vec<ResponseHeader>,
    content_type: Option<String>,
}

impl Head {
    pub(crate) fn of(response: &reqwest::Response) -> Self {
        let headers: Vec<ResponseHeader> = response
            .headers()
            .iter()
            .map(|(name, value)| ResponseHeader {
                name: name.as_str().to_owned(),
                value: String::from_utf8_lossy(value.as_bytes()).into_owned(),
            })
            .collect();
        let content_type = headers
            .iter()
            .find(|h| h.name == "content-type")
            .map(|h| h.value.clone());
        Self {
            status: response.status().as_u16(),
            reason: response.status().canonical_reason().map(str::to_owned),
            http_version: format!("{:?}", response.version()),
            url: response.url().to_string(),
            headers,
            content_type,
        }
    }

    pub(crate) fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    pub(crate) fn into_response(
        self,
        collected: crate::body::Collected,
        timing: Timing,
    ) -> Response {
        let (text, base64) = decode(&collected.head);
        Response {
            status: self.status,
            reason: self.reason,
            http_version: self.http_version,
            url: self.url,
            headers: self.headers,
            body: ResponseBody {
                text,
                base64,
                size: collected.total,
                truncated: collected.truncated,
                file: collected.file.map(|p| p.to_string_lossy().into_owned()),
                content_type: self.content_type,
            },
            timing,
            stream_events: collected.events,
        }
    }
}

/// Text when the bytes are UTF-8 (allowing a character cut at the end), base64 otherwise.
fn decode(bytes: &[u8]) -> (Option<String>, Option<String>) {
    use base64::Engine;
    match std::str::from_utf8(bytes) {
        Ok(text) => (Some(text.to_owned()), None),
        Err(e) if e.error_len().is_none() => (
            Some(String::from_utf8_lossy(&bytes[..e.valid_up_to()]).into_owned()),
            None,
        ),
        Err(_) => (
            None,
            Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
        ),
    }
}
