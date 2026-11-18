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
        settings: None,
        name: None,
        path: None,
        auth: None,
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
                &mut |_| {},
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
    let error = send(
        &state(),
        &deps().0,
        &deps().1,
        &mut |_| {},
        args("tab-1", "nope".into()),
    )
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
        &mut |_| {},
        args("tab-1", format!("{base}/")),
    )
    .await
    .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body.text.as_deref(), Some("ok"));
    assert!(state.in_flight.is_empty());
}

#[test]
fn bodies_are_saved_from_text_base64_or_a_file_of_ours_only() {
    let dir = tempfile::tempdir().unwrap();
    let spill = dir.path().join("bodies");
    let state = AppState::new(SendOptions {
        spill_dir: Some(spill.clone()),
        ..SendOptions::default()
    })
    .unwrap();
    std::fs::create_dir_all(&spill).unwrap();
    std::fs::write(spill.join("body-1.bin"), b"whole body").unwrap();
    let outside = dir.path().join("secret.txt");
    std::fs::write(&outside, b"not a body").unwrap();
    let target = |name: &str| dir.path().join(name).to_string_lossy().into_owned();
    let save = |args: crate::ipc::ResponseSaveArgs| save_response(&state, &args);
    let args = |destination: String| crate::ipc::ResponseSaveArgs {
        destination,
        file: None,
        text: None,
        base64: None,
    };

    save(crate::ipc::ResponseSaveArgs {
        text: Some("hi".into()),
        ..args(target("a.txt"))
    })
    .unwrap();
    save(crate::ipc::ResponseSaveArgs {
        base64: Some("iVD//g==".into()),
        ..args(target("b.bin"))
    })
    .unwrap();
    save(crate::ipc::ResponseSaveArgs {
        file: Some(spill.join("body-1.bin").to_string_lossy().into_owned()),
        ..args(target("c.bin"))
    })
    .unwrap();
    let refused = save(crate::ipc::ResponseSaveArgs {
        file: Some(outside.to_string_lossy().into_owned()),
        ..args(target("d.bin"))
    })
    .unwrap_err();

    assert_eq!(std::fs::read(dir.path().join("a.txt")).unwrap(), b"hi");
    assert_eq!(
        std::fs::read(dir.path().join("b.bin")).unwrap(),
        [0x89, 0x50, 0xff, 0xfe]
    );
    assert_eq!(
        std::fs::read(dir.path().join("c.bin")).unwrap(),
        b"whole body"
    );
    assert_eq!(refused.code, "invalid_path");
}
