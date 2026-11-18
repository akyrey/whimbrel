//! The redirect target of the authorization code flow (RFC 8252 §7.3): a listener on 127.0.0.1
//! for one request, which carries the code (or the server's error) and the state.

use std::collections::HashMap;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::FlowError;

const PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Whimbrel</title>\
<p style=\"font-family:system-ui;margin:3em\">Signed in. You can close this window and go back to Whimbrel.</p>";
const DONE_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Whimbrel</title>\
<p style=\"font-family:system-ui;margin:3em\">You can close this window.</p>";

pub(crate) struct Listener {
    listener: TcpListener,
    port: u16,
}

fn unreachable(error: std::io::Error) -> FlowError {
    FlowError::Unreachable(format!("the local redirect listener: {error}"))
}

impl Listener {
    pub(crate) async fn bind() -> Result<Self, FlowError> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(unreachable)?;
        let port = listener.local_addr().map_err(unreachable)?.port();
        Ok(Self { listener, port })
    }

    pub(crate) fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }

    /// The code of the first request to `/callback`; other paths (a favicon) are answered and
    /// skipped.
    pub(crate) async fn wait(self, state: &str) -> Result<String, FlowError> {
        loop {
            let (mut socket, _) = self.listener.accept().await.map_err(unreachable)?;
            let mut buffer = vec![0u8; 8192];
            let read = socket.read(&mut buffer).await.map_err(unreachable)?;
            let head = String::from_utf8_lossy(&buffer[..read]).to_string();
            let target = head.split_whitespace().nth(1).unwrap_or("").to_owned();
            let Some(query) = target.strip_prefix("/callback") else {
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                    )
                    .await;
                continue;
            };
            let pairs: HashMap<String, String> =
                url::form_urlencoded::parse(query.trim_start_matches('?').as_bytes())
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
        let listener = Listener::bind().await.unwrap();
        let uri = listener.redirect_uri();
        let waiting = tokio::spawn(async move { listener.wait("s1").await });

        let favicon = visit(uri.clone(), "/favicon.ico").await;
        let page = visit(uri, "/callback?code=c1&state=s1").await;

        assert!(favicon.starts_with("HTTP/1.1 404"));
        assert!(page.contains("Signed in"));
        assert_eq!(waiting.await.unwrap().unwrap(), "c1");
    }

    #[tokio::test]
    async fn another_state_or_the_servers_error_is_refused() {
        let listener = Listener::bind().await.unwrap();
        let uri = listener.redirect_uri();
        let waiting = tokio::spawn(async move { listener.wait("mine").await });
        visit(uri, "/callback?code=c1&state=theirs").await;
        assert_eq!(
            waiting.await.unwrap().unwrap_err(),
            FlowError::StateMismatch
        );

        let listener = Listener::bind().await.unwrap();
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
