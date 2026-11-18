//! A tiny HTTP/1.1 server on localhost for the tests: enough to see what was sent and to answer
//! slowly, with redirects, or with odd bodies.
#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// What the server received.
#[derive(Debug, Clone)]
pub struct Seen {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Seen {
    pub fn header(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

async fn read_request(socket: &mut TcpStream) -> Option<Seen> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let read = socket.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        data.extend_from_slice(&chunk[..read]);
        if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
    };
    let head = String::from_utf8_lossy(&data[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_owned();
    let path = first.next()?.to_owned();
    let headers: Vec<(String, String)> = lines
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_once(": "))
        .map(|(n, v)| (n.to_owned(), v.to_owned()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    while data.len() < head_end + length {
        let read = socket.read(&mut chunk).await.ok()?;
        if read == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..read]);
    }
    let body = String::from_utf8_lossy(&data[head_end..]).to_string();
    Some(Seen {
        method,
        path,
        headers,
        body,
    })
}

fn respond(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status}\r\ncontent-length: {}\r\n", body.len());
    for (name, value) in headers {
        out.push_str(&format!("{name}: {value}\r\n"));
    }
    out.push_str("connection: close\r\n\r\n");
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

/// Routes: `/echo` answers with what it saw as JSON, `/slow` waits 5 s, `/redirect` sends to
/// `/echo`, `/big` sends 64 KiB, `/binary` sends non-UTF-8 bytes; anything else is 404.
async fn answer(seen: &Seen) -> Vec<u8> {
    match seen.path.as_str() {
        "/echo" => {
            let json = serde_json::json!({
                "method": seen.method,
                "headers": seen.headers,
                "body": seen.body,
            });
            respond(
                "200 OK",
                &[
                    ("content-type", "application/json"),
                    ("x-trace", "1"),
                    ("x-trace", "2"),
                ],
                json.to_string().as_bytes(),
            )
        }
        "/slow" => {
            tokio::time::sleep(Duration::from_secs(5)).await;
            respond("200 OK", &[], b"late")
        }
        "/redirect" => respond("302 Found", &[("location", "/echo")], b""),
        "/big" => respond("200 OK", &[("content-type", "text/plain")], &[b'x'; 65_536]),
        "/binary" => respond(
            "200 OK",
            &[("content-type", "image/png")],
            &[0x89, 0x50, 0xff, 0xfe],
        ),
        _ => respond(
            "404 Not Found",
            &[("content-type", "text/plain")],
            b"no such route",
        ),
    }
}

/// Starts the server; returns its base URL, e.g. `http://127.0.0.1:54321`.
pub async fn server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                if let Some(seen) = read_request(&mut socket).await {
                    let _ = socket.write_all(&answer(&seen).await).await;
                }
            });
        }
    });
    base
}

/// A local port with nothing listening.
pub async fn closed_port() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    base
}
