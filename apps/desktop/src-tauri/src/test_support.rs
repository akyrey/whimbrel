#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Shared by the tests of the checks and the runner.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const TOKEN: &str = "tok-123";

/// `/login` answers a token; `/me` answers 200 with that token as a bearer, else 401.
pub async fn auth_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
                let (status, body) = if head.contains(" /login ") {
                    ("200 OK", format!(r#"{{"token":"{TOKEN}"}}"#))
                } else if head.contains(&format!("authorization: bearer {TOKEN}")) {
                    ("200 OK", r#"{"name":"ada"}"#.to_owned())
                } else {
                    ("401 Unauthorized", "{}".to_owned())
                };
                let answer = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(answer.as_bytes()).await;
            });
        }
    });
    base
}

/// Accepts connections and reads, but never answers.
pub async fn silent_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                while socket.read(&mut buf).await.unwrap_or(0) > 0 {}
            });
        }
    });
    base
}
