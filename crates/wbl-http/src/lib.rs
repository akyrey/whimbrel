//! Sends a [`wbl_core::Request`] and reads the response: reqwest with rustls and the OS trust
//! store, HTTP/2, decompression, a timeout, redirects, a body cap and cancellation.
//!
//! Timings are the time to the response headers and the total (D-H10: reqwest does not expose
//! DNS, connect and TLS times).

mod error;
mod response;

use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;
use wbl_core::{Body, Request};

pub use error::{HttpError, HttpErrorCode};
pub use response::{Response, ResponseBody, ResponseHeader, Timing};

/// How requests are sent. One [`Sender`] (and its connection pool) serves the whole app.
#[derive(Debug, Clone)]
pub struct SendOptions {
    pub timeout: Duration,
    pub max_redirects: usize,
    /// Bytes of a body kept; the rest is not read and the body is marked truncated.
    pub max_body_bytes: usize,
}

/// Defaults until requests have their own settings (Phase 4).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_REDIRECTS: usize = 10;
const DEFAULT_MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

impl Default for SendOptions {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        }
    }
}

pub struct Sender {
    client: reqwest::Client,
    max_body_bytes: usize,
}

impl Sender {
    pub fn new(options: SendOptions) -> Result<Self, HttpError> {
        let client = reqwest::Client::builder()
            .timeout(options.timeout)
            .redirect(reqwest::redirect::Policy::limited(options.max_redirects))
            .user_agent(concat!("Whimbrel/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| HttpError::new(HttpErrorCode::Http, error::describe(&e)))?;
        Ok(Self {
            client,
            max_body_bytes: options.max_body_bytes,
        })
    }

    /// Sends `request` and reads its response, unless `cancel` fires first.
    pub async fn send(
        &self,
        request: &Request,
        cancel: &CancellationToken,
    ) -> Result<Response, HttpError> {
        request.validate().map_err(HttpError::from)?;
        let built = self.build(request)?;
        tokio::select! {
            () = cancel.cancelled() => Err(HttpError::new(
                HttpErrorCode::Cancelled,
                "The request was cancelled",
            )),
            result = self.exchange(built) => result,
        }
    }

    fn build(&self, request: &Request) -> Result<reqwest::Request, HttpError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|_| HttpError::invalid("method", "The method is not valid"))?;
        let mut builder = self.client.request(method, request.url.trim());
        for header in request.enabled_headers() {
            builder = builder.header(&header.name, &header.value);
        }
        if let Some(content_type) = request.default_content_type() {
            builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
        }
        if let Body::Text { text, .. } = &request.body {
            builder = builder.body(text.clone());
        }
        builder
            .build()
            .map_err(|e| HttpError::new(HttpErrorCode::InvalidRequest, error::describe(&e)))
    }

    async fn exchange(&self, request: reqwest::Request) -> Result<Response, HttpError> {
        let started = Instant::now();
        let response = self
            .client
            .execute(request)
            .await
            .map_err(HttpError::from)?;
        let headers_at = started.elapsed();
        let head = response::Head::of(&response);
        let (bytes, truncated) = read_capped(response, self.max_body_bytes).await?;
        Ok(head.into_response(bytes, truncated, Timing::of(headers_at, started.elapsed())))
    }
}

/// Reads at most `cap` bytes; `true` when the body was longer.
async fn read_capped(
    mut response: reqwest::Response,
    cap: usize,
) -> Result<(Vec<u8>, bool), HttpError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(HttpError::from)? {
        let room = cap.saturating_sub(bytes.len());
        if chunk.len() > room {
            bytes.extend_from_slice(&chunk[..room]);
            return Ok((bytes, true));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok((bytes, false))
}
