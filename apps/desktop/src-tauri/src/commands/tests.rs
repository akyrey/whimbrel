#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use wbl_core::{Body, Request};
use wbl_http::SendOptions;

use super::*;

fn state() -> Arc<AppState> {
    Arc::new(AppState::new(SendOptions::default()).unwrap())
}

fn args(id: &str, url: String) -> RequestSendArgs {
    RequestSendArgs {
        request_id: id.into(),
        request: Request {
            method: "GET".into(),
            url,
            headers: vec![],
            body: Body::None,
        },
        root: None,
        environment: None,
        example: None,
    }
}

fn deps() -> (crate::collections::Collections, crate::secrets::Secrets) {
    (
        crate::collections::Collections::default(),
        crate::secrets::Secrets::new(Arc::new(roost_keychain::MemorySecretStore::default())),
    )
}

/// Accepts connections and reads, but never answers.
async fn silent_server() -> String {
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

#[tokio::test]
async fn cancel_ends_a_running_request_with_cancelled() {
    let state = state();
    let base = silent_server().await;
    let sending = tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            send(
                &state,
                &deps().0,
                &deps().1,
                args("tab-1", format!("{base}/")),
            )
            .await
        }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let cancelled = cancel(
        &state,
        &RequestCancelArgs {
            request_id: "tab-1".into(),
        },
    )
    .unwrap();
    let error = sending.await.unwrap().unwrap_err();

    assert!(cancelled);
    assert_eq!(error.code, "cancelled");
    assert!(state.in_flight.is_empty());
}

#[tokio::test]
async fn an_invalid_request_is_an_app_error_naming_the_field() {
    let error = send(&state(), &deps().0, &deps().1, args("tab-1", "nope".into()))
        .await
        .unwrap_err();

    assert_eq!(error.code, "invalid_request");
    assert_eq!(error.detail, Some(serde_json::json!({ "field": "url" })));
}

#[tokio::test]
async fn cancelling_a_finished_request_says_so() {
    assert!(
        !cancel(
            &state(),
            &RequestCancelArgs {
                request_id: "x".into()
            }
        )
        .unwrap()
    );
}

/// Answers every request with 200 and `ok`.
async fn ok_server() -> String {
    use tokio::io::AsyncWriteExt;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let reply = b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok";
                let _ = socket.write_all(reply).await;
            });
        }
    });
    base
}

#[tokio::test]
async fn a_finished_request_leaves_nothing_in_flight() {
    let state = state();
    let base = ok_server().await;

    let response = send(
        &state,
        &deps().0,
        &deps().1,
        args("tab-1", format!("{base}/")),
    )
    .await
    .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body.text.as_deref(), Some("ok"));
    assert!(state.in_flight.is_empty());
}
