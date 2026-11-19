//! Sends a [`wbl_core::Request`] and reads the response: reqwest with rustls and the OS trust
//! store, HTTP/2, decompression, per-request settings, a cookie jar per collection, large bodies
//! to disk, streams parsed as they arrive, and cancellation.
//!
//! Timings are the time to the response headers and the total (D-H10).

mod body;
mod error;
mod response;
mod stream;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::cookie::Jar;
use tokio_util::sync::CancellationToken;
use wbl_core::{Body, Request, RequestSettings};

pub use error::{HttpError, HttpErrorCode};
pub use response::{Response, ResponseBody, ResponseHeader, Timing};
pub use stream::{StreamEvent, StreamKind};

/// How requests are sent. One [`Sender`] (and its connection pools) serves the whole app.
#[derive(Debug, Clone)]
pub struct SendOptions {
    /// For requests without their own timeout.
    pub timeout: Duration,
    pub max_redirects: usize,
    /// Bytes kept in memory; a longer body goes to `spill_dir` (or is cut without one).
    pub max_body_bytes: usize,
    /// Bytes shown of a body that went to a file.
    pub preview_bytes: usize,
    /// The most written to a file.
    pub max_file_bytes: u64,
    pub spill_dir: Option<PathBuf>,
}

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_REDIRECTS: usize = 10;
const DEFAULT_MAX_BODY_BYTES: usize = 10 * 1024 * 1024;
const DEFAULT_PREVIEW_BYTES: usize = 1024 * 1024;
const DEFAULT_MAX_FILE_BYTES: u64 = 1024 * 1024 * 1024;

impl Default for SendOptions {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            preview_bytes: DEFAULT_PREVIEW_BYTES,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            spill_dir: None,
        }
    }
}

/// A collection's own TLS (FR-5.3), as PEM bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TlsFiles {
    /// Certificates trusted besides the system's.
    pub ca_pem: Option<Vec<u8>>,
    /// A client certificate and its private key.
    pub client_pem: Option<Vec<u8>>,
}

/// What one send needs besides the request.
#[derive(Debug, Clone, Default)]
pub struct Exchange {
    pub settings: RequestSettings,
    /// The cookie jar to use (one per collection); `None`: no cookies kept.
    pub jar: Option<String>,
    pub tls: TlsFiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ClientKey {
    jar: Option<String>,
    follow_redirects: bool,
    verify_tls: bool,
    tls: TlsFiles,
}

fn tls_error(what: &str, error: &reqwest::Error) -> HttpError {
    HttpError::new(
        HttpErrorCode::Tls,
        format!(
            "The collection's {what} is not usable PEM ({})",
            error::describe(error)
        ),
    )
}

fn with_tls(
    mut builder: reqwest::ClientBuilder,
    tls: &TlsFiles,
) -> Result<reqwest::ClientBuilder, HttpError> {
    if let Some(pem) = &tls.ca_pem {
        let certificates =
            reqwest::Certificate::from_pem_bundle(pem).map_err(|e| tls_error("CA file", &e))?;
        if certificates.is_empty() {
            return Err(HttpError::new(
                HttpErrorCode::Tls,
                "The collection's CA file holds no certificate",
            ));
        }
        for certificate in certificates {
            builder = builder.add_root_certificate(certificate);
        }
    }
    if let Some(pem) = &tls.client_pem {
        builder = builder.identity(
            reqwest::Identity::from_pem(pem).map_err(|e| tls_error("client certificate", &e))?,
        );
    }
    Ok(builder)
}

pub struct Sender {
    options: SendOptions,
    clients: Mutex<HashMap<ClientKey, reqwest::Client>>,
    jars: Mutex<HashMap<String, Arc<Jar>>>,
}

fn poisoned() -> HttpError {
    HttpError::new(
        HttpErrorCode::Http,
        "The connection pool is unusable; restart the app",
    )
}

impl Sender {
    pub fn new(options: SendOptions) -> Result<Self, HttpError> {
        let sender = Self {
            options,
            clients: Mutex::new(HashMap::new()),
            jars: Mutex::new(HashMap::new()),
        };
        // Fail at start, not at the first send, if the TLS stack cannot be set up.
        sender.client(&Exchange::default())?;
        Ok(sender)
    }

    fn client(&self, exchange: &Exchange) -> Result<reqwest::Client, HttpError> {
        let key = ClientKey {
            jar: exchange.jar.clone(),
            follow_redirects: exchange.settings.follow_redirects,
            verify_tls: exchange.settings.verify_tls,
            tls: exchange.tls.clone(),
        };
        let mut clients = self.clients.lock().map_err(|_| poisoned())?;
        if let Some(client) = clients.get(&key) {
            return Ok(client.clone());
        }
        let redirects = if key.follow_redirects {
            reqwest::redirect::Policy::limited(self.options.max_redirects)
        } else {
            reqwest::redirect::Policy::none()
        };
        let mut builder = reqwest::Client::builder()
            .timeout(self.options.timeout)
            .redirect(redirects)
            .danger_accept_invalid_certs(!key.verify_tls)
            .user_agent(concat!("Whimbrel/", env!("CARGO_PKG_VERSION")));
        builder = with_tls(builder, &key.tls)?;
        if let Some(name) = &key.jar {
            let jar = Arc::clone(
                self.jars
                    .lock()
                    .map_err(|_| poisoned())?
                    .entry(name.clone())
                    .or_default(),
            );
            builder = builder.cookie_provider(jar);
        }
        let client = builder
            .build()
            .map_err(|e| HttpError::new(HttpErrorCode::Http, error::describe(&e)))?;
        clients.insert(key, client.clone());
        Ok(client)
    }

    /// Sends with the default settings and no cookie jar.
    pub async fn send(
        &self,
        request: &Request,
        cancel: &CancellationToken,
    ) -> Result<Response, HttpError> {
        self.send_with(request, &Exchange::default(), cancel, &mut |_| {})
            .await
    }

    /// Sends `request`; stream events go to `on_event` as they arrive.
    pub async fn send_with(
        &self,
        request: &Request,
        exchange: &Exchange,
        cancel: &CancellationToken,
        on_event: &mut (dyn FnMut(StreamEvent) + Send),
    ) -> Result<Response, HttpError> {
        request.validate().map_err(HttpError::from)?;
        let client = self.client(exchange)?;
        let files = read_files(&request.body).await?;
        let built = build(&client, request, &exchange.settings, &files)?;
        tokio::select! {
            () = cancel.cancelled() => Err(HttpError::new(HttpErrorCode::Cancelled, "The request was cancelled")),
            result = self.exchange(&client, built, on_event) => result,
        }
    }

    async fn exchange(
        &self,
        client: &reqwest::Client,
        request: reqwest::Request,
        on_event: &mut (dyn FnMut(StreamEvent) + Send),
    ) -> Result<Response, HttpError> {
        let started = Instant::now();
        let response = client.execute(request).await.map_err(HttpError::from)?;
        let headers_at = started.elapsed();
        let head = response::Head::of(&response);
        let parser = StreamKind::of(head.content_type()).map(stream::Parser::new);
        let collected = body::collect(response, &self.options, parser, on_event).await?;
        Ok(head.into_response(collected, Timing::of(headers_at, started.elapsed())))
    }
}

/// The contents of every file the body sends, by path.
async fn read_files(body: &Body) -> Result<HashMap<String, Vec<u8>>, HttpError> {
    let paths: Vec<&str> = match body {
        Body::File { path } => vec![path.as_str()],
        Body::Multipart { .. } => body
            .enabled_fields()
            .filter(|f| f.file)
            .map(|f| f.value.as_str())
            .collect(),
        _ => Vec::new(),
    };
    let mut files = HashMap::new();
    for path in paths {
        let bytes = tokio::fs::read(path).await.map_err(|e| {
            HttpError::invalid("body", &format!("Cannot read the file {path}: {e}"))
        })?;
        files.insert(path.to_owned(), bytes);
    }
    Ok(files)
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map_or_else(|| "file".to_owned(), |n| n.to_string_lossy().into_owned())
}

fn multipart(body: &Body, files: &HashMap<String, Vec<u8>>) -> reqwest::multipart::Form {
    body.enabled_fields()
        .fold(reqwest::multipart::Form::new(), |form, f| {
            if f.file {
                let bytes = files.get(&f.value).cloned().unwrap_or_default();
                form.part(
                    f.name.clone(),
                    reqwest::multipart::Part::bytes(bytes).file_name(file_name(&f.value)),
                )
            } else {
                form.text(f.name.clone(), f.value.clone())
            }
        })
}

fn with_body(
    builder: reqwest::RequestBuilder,
    body: &Body,
    files: &HashMap<String, Vec<u8>>,
) -> reqwest::RequestBuilder {
    match body {
        Body::None => builder,
        Body::Text { text, .. } => builder.body(text.clone()),
        Body::Form { .. } => {
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(
                    body.enabled_fields()
                        .map(|f| (f.name.as_str(), f.value.as_str())),
                )
                .finish();
            builder.body(encoded)
        }
        Body::Multipart { .. } => builder.multipart(multipart(body, files)),
        Body::File { path } => builder.body(files.get(path).cloned().unwrap_or_default()),
    }
}

fn build(
    client: &reqwest::Client,
    request: &Request,
    settings: &RequestSettings,
    files: &HashMap<String, Vec<u8>>,
) -> Result<reqwest::Request, HttpError> {
    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|_| HttpError::invalid("method", "The method is not valid"))?;
    let mut builder = client.request(method, request.url.trim());
    if let Some(ms) = settings.timeout_ms {
        builder = builder.timeout(Duration::from_millis(ms));
    }
    for header in request.enabled_headers() {
        builder = builder.header(&header.name, &header.value);
    }
    if let Some(content_type) = request.default_content_type() {
        builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
    }
    builder = with_body(builder, &request.body, files);
    builder
        .build()
        .map_err(|e| HttpError::new(HttpErrorCode::InvalidRequest, error::describe(&e)))
}
