#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use wbl_core::auth::{Auth, Grant, OAuth2Config};
use wbl_core::{Body, Request};
use wbl_format::collection::{self, auth};
use wbl_http::SendOptions;

use super::*;
use crate::commands::{preview, send};
use crate::ipc::{CollectionCreateArgs, RequestSendArgs};

/// Answers `POST /token` with a new token (`access-N`) and every other request with the
/// Authorization header it got, recorded in `seen`.
async fn server() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    tokio::spawn(async move {
        let mut issued = 0;
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buffer = vec![0u8; 8192];
            let read = socket.read(&mut buffer).await.unwrap_or(0);
            let head = String::from_utf8_lossy(&buffer[..read]).to_string();
            let body = if head.starts_with("POST /token") {
                issued += 1;
                format!(
                    r#"{{"access_token":"access-{issued}","expires_in":3600,"refresh_token":"refresh-{issued}"}}"#
                )
            } else {
                let auth = head
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("authorization: ")
                            .or_else(|| l.strip_prefix("Authorization: "))
                    })
                    .unwrap_or("")
                    .to_owned();
                record.lock().unwrap().push(auth.clone());
                auth
            };
            let reply = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(reply.as_bytes()).await;
        }
    });
    (base, seen)
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: String,
    collections: Collections,
    secrets: Secrets,
    state: AppState,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let collections = Collections::default();
    collections
        .create(&CollectionCreateArgs {
            root: root.clone(),
            name: "API".into(),
        })
        .unwrap();
    Fixture {
        _dir: dir,
        root,
        collections,
        secrets: Secrets::new(Arc::new(roost_keychain::MemorySecretStore::default())),
        state: AppState::new(SendOptions::default()).unwrap(),
    }
}

fn args(f: &Fixture, url: String, path: Option<&str>) -> RequestSendArgs {
    RequestSendArgs {
        request_id: "tab".into(),
        request: Request {
            method: "GET".into(),
            url,
            headers: vec![],
            body: Body::None,
        },
        root: Some(f.root.clone()),
        environment: None,
        example: None,
        settings: None,
        name: None,
        path: path.map(str::to_owned),
        auth: None,
        variables: None,
        captures: None,
        assertions: None,
    }
}

fn oauth(base: &str) -> Auth {
    Auth::OAuth2(OAuth2Config {
        grant: Grant::ClientCredentials,
        token_url: format!("{base}/token"),
        auth_url: String::new(),
        device_url: String::new(),
        client_id: "app".into(),
        client_secret: "shh".into(),
        scope: String::new(),
    })
}

async fn sent(f: &Fixture, a: RequestSendArgs) -> Result<wbl_http::Response, AppError> {
    send(&f.state, &f.collections, &f.secrets, &mut |_| {}, a)
        .await
        .map(|sent| sent.response)
}

#[tokio::test]
async fn a_folder_auth_is_inherited_and_masked_where_shown() {
    let f = fixture();
    let (base, seen) = server().await;
    let root = Path::new(&f.root);
    let folder = collection::create_folder(root, "", "Users").unwrap();
    auth::save_auth(
        root,
        &folder,
        &Auth::Basic {
            username: "ada".into(),
            password: "s3cret".into(),
        },
    )
    .unwrap();
    let a = args(
        &f,
        format!("{base}/users"),
        Some(&format!("{folder}/list.req.toml")),
    );

    sent(&f, a.clone()).await.unwrap();
    let shown = preview(&f.collections, &f.secrets, &a).unwrap();

    assert_eq!(seen.lock().unwrap().as_slice(), ["Basic YWRhOnMzY3JldA=="]);
    assert_eq!(shown.headers[0].value, "Basic ••••••");
    let own = RequestSendArgs {
        auth: Some(Auth::None),
        ..a
    };
    assert!(
        preview(&f.collections, &f.secrets, &own)
            .unwrap()
            .headers
            .is_empty(),
        "the request's own auth wins"
    );
}

#[tokio::test]
async fn oauth_needs_a_token_then_sends_it_masked_and_forgets_it() {
    let f = fixture();
    let (base, seen) = server().await;
    auth::save_auth(Path::new(&f.root), "", &oauth(&base)).unwrap();
    let a = args(&f, format!("{base}/data"), None);
    let context = crate::commands::parts(&f.collections, &f.secrets, &a)
        .unwrap()
        .auth;

    let missing = sent(&f, a.clone()).await.unwrap_err();
    let before = status(&f.secrets, &context).unwrap();
    let after = authorize(
        &f.state.oauth,
        &f.secrets,
        &context,
        &CancellationToken::new(),
        |_| Ok(()),
        |_| {},
    )
    .await
    .unwrap();
    sent(&f, a.clone()).await.unwrap();
    let shown = preview(&f.collections, &f.secrets, &a).unwrap();
    forget(&f.secrets, &context).unwrap();

    assert_eq!(missing.code, "oauth_required");
    assert!(before.configured && !before.has_token);
    assert!(after.has_token && after.can_refresh);
    assert_eq!(
        seen.lock().unwrap().last().map(String::as_str),
        Some("Bearer access-1")
    );
    assert_eq!(shown.headers[0].value, "Bearer ••••••");
    assert!(!status(&f.secrets, &context).unwrap().has_token);
}

#[tokio::test]
async fn a_stale_token_is_refreshed_and_kept() {
    let f = fixture();
    let (base, seen) = server().await;
    auth::save_auth(Path::new(&f.root), "", &oauth(&base)).unwrap();
    let a = args(&f, format!("{base}/data"), None);
    let context = crate::commands::parts(&f.collections, &f.secrets, &a)
        .unwrap()
        .auth;
    let config = config_of(&context).unwrap().clone();
    let stale = Token {
        access_token: "old".into(),
        refresh_token: Some("refresh-0".into()),
        expires_at_ms: Some(1),
        scope: None,
    };
    store(&f.secrets, &context, &config, &stale).unwrap();

    sent(&f, a).await.unwrap();

    assert_eq!(
        seen.lock().unwrap().last().map(String::as_str),
        Some("Bearer access-1")
    );
    assert_eq!(
        cached(&f.secrets, &context).unwrap().unwrap().access_token,
        "access-1"
    );
}
