#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use wbl_format::collection::auth;

use super::tokens::Tokens;
use super::*;
use crate::send::parts;
use crate::test_support::{Fixture, fixture};

/// Answers `POST /token` with `access-N` (refreshable) and every other request with the
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
                    .find_map(|l| l.strip_prefix("authorization: "))
                    .unwrap_or("")
                    .to_owned();
                record.lock().unwrap().push(auth.clone());
                auth
            };
            let reply = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(reply.as_bytes()).await;
        }
    });
    (base, seen)
}

fn config(base: &str) -> OAuth2Config {
    OAuth2Config {
        grant: Grant::ClientCredentials,
        token_url: format!("{base}/token"),
        auth_url: String::new(),
        device_url: String::new(),
        client_id: "app".into(),
        client_secret: "shh".into(),
        scope: String::new(),
        redirect_uri: String::new(),
    }
}

fn context(f: &Fixture, base: &str) -> AuthContext {
    let root = f.root();
    auth::save_auth(&root, "", &Auth::OAuth2(config(base))).unwrap();
    parts(&f.sources(&root), &f.args(&format!("{base}/data")))
        .unwrap()
        .auth
}

async fn sign_in(f: &Fixture, context: &AuthContext, name: Option<&str>) -> OAuthStatus {
    authorize(
        &f.oauth,
        &f.secrets,
        context,
        name,
        &CancellationToken::new(),
        |_| Ok(()),
        |_| {},
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn without_a_token_the_send_is_refused_then_a_sign_in_is_used() {
    let f = fixture();
    let (base, seen) = server().await;
    let context = context(&f, &base);

    let refused = f.send(f.args(&format!("{base}/data"))).await.unwrap_err();
    let signed_in = sign_in(&f, &context, Some("Service")).await;
    f.send(f.args(&format!("{base}/data"))).await.unwrap();

    assert_eq!(refused.code, "oauth_required");
    assert!(signed_in.configured && signed_in.has_token && signed_in.can_refresh);
    assert_eq!(signed_in.tokens[0].name, "Service");
    assert_eq!(
        seen.lock().unwrap().last().map(String::as_str),
        Some("Bearer access-1")
    );
}

#[tokio::test]
async fn a_stale_token_is_refreshed_and_keeps_its_name() {
    let f = fixture();
    let (base, _) = server().await;
    let context = context(&f, &base);
    let tokens = Tokens::new(&f.secrets, &context.collection_id, &config(&base));
    let stale = Token {
        access_token: "old".into(),
        refresh_token: Some("refresh-0".into()),
        expires_at_ms: Some(1),
        scope: None,
    };
    tokens
        .add(&tokens.index().unwrap(), Some("Kept"), &stale, 1)
        .unwrap();

    let used = usable(&f.oauth, &f.secrets, &context).await.unwrap();

    assert_eq!(used.unwrap().access_token, "access-1");
    assert_eq!(
        cached(&f.secrets, &context).unwrap().unwrap().access_token,
        "access-1"
    );
    assert_eq!(status(&f.secrets, &context).unwrap().tokens[0].name, "Kept");
}

#[tokio::test]
async fn a_stale_token_without_refresh_needs_a_sign_in() {
    let f = fixture();
    let (base, _) = server().await;
    let context = context(&f, &base);
    let tokens = Tokens::new(&f.secrets, &context.collection_id, &config(&base));
    let stale = Token {
        access_token: "old".into(),
        refresh_token: None,
        expires_at_ms: Some(1),
        scope: None,
    };
    tokens
        .add(&tokens.index().unwrap(), None, &stale, 1)
        .unwrap();

    assert!(
        usable(&f.oauth, &f.secrets, &context)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn tokens_can_be_renamed_switched_and_removed() {
    let f = fixture();
    let (base, _) = server().await;
    let context = context(&f, &base);
    sign_in(&f, &context, Some("First")).await;
    let both = sign_in(&f, &context, None).await;
    let first = both.tokens[0].id.clone();

    let renamed = change(
        &f.secrets,
        &context,
        &TokenChange::Rename {
            id: first.clone(),
            name: "Admin".into(),
        },
    )
    .unwrap();
    let selected = change(
        &f.secrets,
        &context,
        &TokenChange::Select { id: first.clone() },
    )
    .unwrap();
    let removed = change(&f.secrets, &context, &TokenChange::Remove { id: first }).unwrap();

    assert_eq!(renamed.tokens[0].name, "Admin");
    assert!(selected.tokens[0].active);
    assert_eq!(
        cached(&f.secrets, &context).unwrap(),
        None,
        "removing the token in use leaves none in use"
    );
    assert_eq!(removed.tokens.len(), 1);
}

#[tokio::test]
async fn a_bad_name_is_refused_before_signing_in() {
    let f = fixture();
    let (base, _) = server().await;
    let context = context(&f, &base);
    let long = "x".repeat(MAX_NAME_CHARS + 1);

    let error = authorize(
        &f.oauth,
        &f.secrets,
        &context,
        Some(&long),
        &CancellationToken::new(),
        |_| Ok(()),
        |_| {},
    )
    .await
    .unwrap_err();

    assert_eq!(error.code, "invalid_argument");
    assert!(status(&f.secrets, &context).unwrap().tokens.is_empty());
}

#[tokio::test]
async fn a_request_without_oauth_has_no_tokens() {
    let f = fixture();
    let root = f.root();
    let context = parts(&f.sources(&root), &f.args("http://x")).unwrap().auth;

    let state = status(&f.secrets, &context).unwrap();
    let error = change(
        &f.secrets,
        &context,
        &TokenChange::Select { id: "a".into() },
    )
    .unwrap_err();

    assert!(!state.configured && state.tokens.is_empty());
    assert_eq!(cached(&f.secrets, &context).unwrap(), None);
    assert!(
        usable(&f.oauth, &f.secrets, &context)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(error.code, "invalid_argument");
}
