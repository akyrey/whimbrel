//! The redirect target of the authorization code flow (RFC 8252 §7.3): a listener on a loopback
//! address for one request, which carries the code (or the server's error) and the state. The
//! port is random unless the configuration names the redirect URI registered with the server.

use std::collections::HashMap;
use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use wbl_core::auth::loopback_redirect;

use crate::FlowError;

const PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Whimbrel</title>\
<p style=\"font-family:system-ui;margin:3em\">Signed in. You can close this window and go back to Whimbrel.</p>";
const DONE_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Whimbrel</title>\
<p style=\"font-family:system-ui;margin:3em\">You can close this window.</p>";

const RANDOM_PATH: &str = "/callback";

pub(crate) struct Listener {
    listener: TcpListener,
    redirect_uri: String,
    /// The path the redirect arrives on; requests to others are skipped.
    path: String,
}

fn failed(error: std::io::Error) -> FlowError {
    FlowError::Redirect(format!("The local redirect listener failed: {error}"))
}

fn bind_failed(address: SocketAddr, error: std::io::Error) -> FlowError {
    if error.kind() == std::io::ErrorKind::AddrInUse {
        return FlowError::Redirect(format!(
            "Port {} on {} is in use by another program: close it, or register another redirect URI",
            address.port(),
            address.ip()
        ));
    }
    FlowError::Redirect(format!(
        "Whimbrel cannot listen on {address} for the redirect: {error}"
    ))
}

impl Listener {
    /// Listens where `registered` points, or on a random port when it is empty.
    pub(crate) async fn bind(registered: &str) -> Result<Self, FlowError> {
        let registered = registered.trim();
        if registered.is_empty() {
            return Self::bind_random().await;
        }
        let target =
            loopback_redirect(registered).map_err(|e| FlowError::Endpoint(e.to_string()))?;
        let address = SocketAddr::new(target.ip, target.port);
        let listener = TcpListener::bind(address)
            .await
            .map_err(|e| bind_failed(address, e))?;
        Ok(Self {
            listener,
            redirect_uri: registered.to_owned(),
            path: target.path,
        })
    }

    async fn bind_random() -> Result<Self, FlowError> {
        let address = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(address)
            .await
            .map_err(|e| bind_failed(address, e))?;
        let port = listener.local_addr().map_err(failed)?.port();
        Ok(Self {
            listener,
            redirect_uri: format!("http://127.0.0.1:{port}{RANDOM_PATH}"),
            path: RANDOM_PATH.to_owned(),
        })
    }

    /// Sent to the server as given, since it compares it with the registered one.
    pub(crate) fn redirect_uri(&self) -> String {
        self.redirect_uri.clone()
    }

    /// The code of the first request to the redirect's path; other paths (a favicon) are
    /// answered and skipped.
    pub(crate) async fn wait(self, state: &str) -> Result<String, FlowError> {
        loop {
            let (mut socket, _) = self.listener.accept().await.map_err(failed)?;
            let mut buffer = vec![0u8; 8192];
            let read = socket.read(&mut buffer).await.map_err(failed)?;
            let head = String::from_utf8_lossy(&buffer[..read]).to_string();
            let target = head.split_whitespace().nth(1).unwrap_or("").to_owned();
            let (path, query) = target.split_once('?').unwrap_or((target.as_str(), ""));
            if path != self.path {
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                    )
                    .await;
                continue;
            };
            let pairs: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();
            let page = if pairs.contains_key("code") {
                PAGE
            } else {
                DONE_PAGE
            };
            let reply = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{page}",
                page.len()
            );
            let _ = socket.write_all(reply.as_bytes()).await;
            if pairs.get("state").map(String::as_str) != Some(state) {
                return Err(FlowError::StateMismatch);
            }
            if let Some(error) = pairs.get("error") {
                return Err(FlowError::Server {
                    error: error.clone(),
                    description: pairs.get("error_description").cloned(),
                });
            }
            return pairs
                .get("code")
                .cloned()
                .ok_or_else(|| FlowError::Invalid("the redirect has no code".into()));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    async fn visit(uri: String, path: &str) -> String {
        let address = uri
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap()
            .to_owned();
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        socket
            .write_all(format!("GET {path} HTTP/1.1\r\nhost: x\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let mut reply = String::new();
        let _ = socket.read_to_string(&mut reply).await;
        reply
    }

    #[tokio::test]
    async fn a_favicon_is_skipped_and_the_callback_gives_the_code() {
        let listener = Listener::bind("").await.unwrap();
        let uri = listener.redirect_uri();
        let waiting = tokio::spawn(async move { listener.wait("s1").await });

        let favicon = visit(uri.clone(), "/favicon.ico").await;
        let page = visit(uri, "/callback?code=c1&state=s1").await;

        assert!(favicon.starts_with("HTTP/1.1 404"));
        assert!(page.contains("Signed in"));
        assert_eq!(waiting.await.unwrap().unwrap(), "c1");
    }

    #[tokio::test]
    async fn a_registered_redirect_answers_only_its_own_path() {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let listener = Listener::bind(&format!(" http://localhost:{port}/oauth/cb "))
            .await
            .unwrap();
        let uri = listener.redirect_uri();
        let waiting = tokio::spawn(async move { listener.wait("s1").await });

        let longer = visit(uri.clone(), "/oauth/cbx?code=wrong&state=s1").await;
        let page = visit(uri.clone(), "/oauth/cb?app=1&code=c1&state=s1").await;

        assert_eq!(uri, format!("http://localhost:{port}/oauth/cb"));
        assert!(longer.starts_with("HTTP/1.1 404"));
        assert!(page.contains("Signed in"));
        assert_eq!(waiting.await.unwrap().unwrap(), "c1");
    }

    #[tokio::test]
    async fn another_state_or_the_servers_error_is_refused() {
        let listener = Listener::bind("").await.unwrap();
        let uri = listener.redirect_uri();
        let waiting = tokio::spawn(async move { listener.wait("mine").await });
        visit(uri, "/callback?code=c1&state=theirs").await;
        assert_eq!(
            waiting.await.unwrap().unwrap_err(),
            FlowError::StateMismatch
        );

        let listener = Listener::bind("").await.unwrap();
        let uri = listener.redirect_uri();
        let waiting = tokio::spawn(async move { listener.wait("s").await });
        let page = visit(
            uri,
            "/callback?error=access_denied&error_description=No&state=s",
        )
        .await;
        assert!(page.contains("You can close this window."));
        assert!(
            matches!(waiting.await.unwrap().unwrap_err(), FlowError::Server { error, .. } if error == "access_denied")
        );
    }
}
