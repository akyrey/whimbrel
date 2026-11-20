#![allow(clippy::unwrap_used, clippy::expect_used)]
//! A server over an in-memory pipe and a client to call its tools.

use std::sync::Arc;

use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::{RoleClient, RunningService};
use serde_json::Value;
use wbl_engine::secrets::Secrets;

use crate::Server;

pub struct Harness {
    pub dirs: Vec<tempfile::TempDir>,
    pub secrets: Secrets,
    pub client: RunningService<RoleClient, ()>,
}

/// One collection per name, a memory keychain, a running server and a connected client.
pub async fn harness(names: &[&str]) -> Harness {
    let dirs: Vec<tempfile::TempDir> = names
        .iter()
        .map(|name| {
            let dir = tempfile::tempdir().unwrap();
            wbl_format::collection::create(dir.path(), name).unwrap();
            dir
        })
        .collect();
    let store = Arc::new(roost_keychain::MemorySecretStore::default());
    let folders: Vec<_> = dirs.iter().map(|d| d.path().to_path_buf()).collect();
    let server = Server::new(&folders, Secrets::new(store.clone())).unwrap();
    let (server_side, client_side) = tokio::io::duplex(1 << 20);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_side).await {
            let _ = running.waiting().await;
        }
    });
    let client = ().serve(client_side).await.unwrap();
    Harness {
        dirs,
        secrets: Secrets::new(store),
        client,
    }
}

impl Harness {
    pub fn root(&self, index: usize) -> std::path::PathBuf {
        self.dirs[index].path().canonicalize().unwrap()
    }

    pub async fn call_raw(&self, tool: &str, arguments: Value) -> CallToolResult {
        let params = CallToolRequestParams::new(tool.to_owned());
        let params = match arguments {
            Value::Object(map) => params.with_arguments(map),
            _ => params,
        };
        self.client.call_tool(params).await.unwrap()
    }

    /// The structured result of a call that succeeded.
    pub async fn call(&self, tool: &str, arguments: Value) -> Value {
        let result = self.call_raw(tool, arguments).await;
        assert_ne!(result.is_error, Some(true), "{tool} failed: {result:?}");
        result.structured_content.unwrap()
    }

    /// The error code of a call that failed.
    pub async fn fails(&self, tool: &str, arguments: Value) -> String {
        let result = self.call_raw(tool, arguments).await;
        assert_eq!(result.is_error, Some(true), "{tool} succeeded: {result:?}");
        result.structured_content.unwrap()["code"]
            .as_str()
            .unwrap()
            .to_owned()
    }
}

pub const TOKEN: &str = "tok-sentinel-31";

/// `/login` answers a token; `/me` answers 200 with that token as a bearer, else 401; `/echo`
/// answers the request it got.
pub async fn server() -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 16384];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).into_owned();
                let lower = head.to_ascii_lowercase();
                let (status, body) = if lower.contains(" /login ") {
                    ("200 OK", format!(r#"{{"token":"{TOKEN}"}}"#))
                } else if lower.contains(" /echo") {
                    ("200 OK", head.clone())
                } else if lower.contains(&format!("authorization: bearer {TOKEN}")) {
                    ("200 OK", r#"{"name":"ada"}"#.to_owned())
                } else {
                    ("401 Unauthorized", "{}".to_owned())
                };
                let reply = format!(
                    "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(reply.as_bytes()).await;
            });
        }
    });
    base
}
