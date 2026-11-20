#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Shared by the engine's tests: a collection in a temporary folder, a memory keychain, and
//! local servers.

use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use wbl_core::{Body, Request};
use wbl_http::{SendOptions, Sender};

use crate::args::RequestSendArgs;
use crate::checks::{Sent, after_send};
use crate::error::EngineError;
use crate::runtime::Runtime;
use crate::secrets::Secrets;
use crate::send::{Sources, ready};

pub const TOKEN: &str = "tok-123";

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub runtime: Runtime,
    pub secrets: Secrets,
    pub oauth: wbl_auth::OAuthClient,
    pub sender: Sender,
}

pub fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    wbl_format::collection::create(&root, "API").unwrap();
    Fixture {
        dir,
        runtime: Runtime::default(),
        secrets: Secrets::new(Arc::new(roost_keychain::MemorySecretStore::default())),
        oauth: wbl_auth::OAuthClient::new(),
        sender: Sender::new(SendOptions::default()).unwrap(),
    }
}

impl Fixture {
    pub fn root(&self) -> std::path::PathBuf {
        self.dir.path().canonicalize().unwrap()
    }

    pub fn root_str(&self) -> String {
        self.root().to_string_lossy().into_owned()
    }

    pub fn collection_id(&self) -> String {
        wbl_format::collection::id(&self.root()).unwrap()
    }

    pub fn sources<'a>(&'a self, root: &'a Path) -> Sources<'a> {
        Sources {
            root: Some(root),
            runtime: &self.runtime,
            secrets: &self.secrets,
        }
    }

    /// A GET of `url` in the collection.
    pub fn args(&self, url: &str) -> RequestSendArgs {
        RequestSendArgs {
            request_id: "tab".into(),
            request: Request {
                method: "GET".into(),
                url: url.into(),
                headers: vec![],
                body: Body::None,
            },
            root: Some(self.root_str()),
            environment: None,
            example: None,
            settings: None,
            name: None,
            path: None,
            auth: None,
            variables: None,
            captures: None,
            assertions: None,
        }
    }

    /// Sends as the app does, without its registry and history.
    pub async fn send(&self, args: RequestSendArgs) -> Result<Sent, EngineError> {
        let root = self.root();
        let sources = self.sources(&root);
        let ready = ready(&self.oauth, &sources, &args).await?;
        let response = self
            .sender
            .send_with(
                &ready.resolved.request,
                &ready.exchange,
                &CancellationToken::new(),
                &mut |_| {},
            )
            .await?;
        let checks = after_send(&self.runtime, &self.secrets, Some(&root), &args, &response)?;
        Ok(Sent { response, checks })
    }
}

/// Answers every request with 200 and the request head and body it got, as text.
pub async fn echo_server() -> String {
    serve(|head| ("200 OK", head.to_owned())).await
}

/// `/login` answers a token; `/me` answers 200 with that token as a bearer, else 401.
pub async fn auth_server() -> String {
    serve(|head| {
        let lower = head.to_ascii_lowercase();
        if lower.contains(" /login ") {
            ("200 OK", format!(r#"{{"token":"{TOKEN}"}}"#))
        } else if lower.contains(&format!(
            "authorization: bearer {}",
            TOKEN.to_ascii_lowercase()
        )) {
            ("200 OK", r#"{"name":"ada"}"#.to_owned())
        } else {
            ("401 Unauthorized", "{}".to_owned())
        }
    })
    .await
}

async fn serve(answer: fn(&str) -> (&'static str, String)) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 16384];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).into_owned();
                let (status, body) = answer(&head);
                let reply = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(reply.as_bytes()).await;
            });
        }
    });
    base
}
