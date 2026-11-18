//! What came back, ready for the frontend.

use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResponseHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    /// The body as text, when it is valid UTF-8; `None` for binary bodies (shown as hex in
    /// Phase 4).
    pub text: Option<String>,
    /// Bytes kept (after decompression), at most the cap.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    /// The body was longer than the cap; the rest was not read.
    pub truncated: bool,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    /// From sending to the response headers.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub headers_ms: u64,
    /// From sending to the end of the body.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub total_ms: u64,
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

impl Timing {
    pub(crate) fn of(headers: Duration, total: Duration) -> Self {
        Self {
            headers_ms: millis(headers),
            total_ms: millis(total),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

    pub(crate) fn into_response(self, bytes: Vec<u8>, truncated: bool, timing: Timing) -> Response {
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        Response {
            status: self.status,
            reason: self.reason,
            http_version: self.http_version,
            url: self.url,
            headers: self.headers,
            body: ResponseBody {
                text: String::from_utf8(bytes).ok(),
                size,
                truncated,
                content_type: self.content_type,
            },
            timing,
        }
    }
}
