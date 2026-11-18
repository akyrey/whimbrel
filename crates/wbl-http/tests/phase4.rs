#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::time::Duration;

use tokio_util::sync::CancellationToken;
use wbl_core::{Body, Request, RequestSettings};
use wbl_http::{Exchange, HttpErrorCode, SendOptions, Sender, StreamEvent};

fn get(url: String) -> Request {
    Request {
        method: "GET".into(),
        url,
        headers: vec![],
        body: Body::None,
    }
}

fn ignore(_: StreamEvent) {}

#[tokio::test]
async fn a_long_body_is_written_whole_to_a_file_and_its_start_is_shown() {
    let base = support::server().await;
    let dir = tempfile::tempdir().unwrap();
    let sender = Sender::new(SendOptions {
        max_body_bytes: 1000,
        preview_bytes: 100,
        spill_dir: Some(dir.path().to_path_buf()),
        ..SendOptions::default()
    })
    .unwrap();

    let response = sender
        .send_with(
            &get(format!("{base}/big")),
            &Exchange::default(),
            &CancellationToken::new(),
            &mut ignore,
        )
        .await
        .unwrap();

    let file = response.body.file.clone().unwrap();
    assert_eq!(response.body.size, 65_536);
    assert!(response.body.truncated);
    assert_eq!(response.body.text.as_deref().map(str::len), Some(100));
    assert_eq!(std::fs::read(&file).unwrap().len(), 65_536);
    assert!(file.starts_with(&dir.path().to_string_lossy().into_owned()));
}

#[tokio::test]
async fn binary_bodies_come_as_base64() {
    let base = support::server().await;
    let response = Sender::new(SendOptions::default())
        .unwrap()
        .send(&get(format!("{base}/binary")), &CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(response.body.text, None);
    assert_eq!(response.body.base64.as_deref(), Some("iVD//g=="));
}

#[tokio::test]
async fn redirects_can_be_left_unfollowed_and_a_request_has_its_own_timeout() {
    let base = support::server().await;
    let sender = Sender::new(SendOptions::default()).unwrap();
    let no_follow = Exchange {
        settings: RequestSettings {
            follow_redirects: false,
            ..RequestSettings::default()
        },
        ..Exchange::default()
    };
    let quick = Exchange {
        settings: RequestSettings {
            timeout_ms: Some(150),
            ..RequestSettings::default()
        },
        ..Exchange::default()
    };

    let redirect = sender
        .send_with(
            &get(format!("{base}/redirect")),
            &no_follow,
            &CancellationToken::new(),
            &mut ignore,
        )
        .await
        .unwrap();
    let late = sender
        .send_with(
            &get(format!("{base}/slow")),
            &quick,
            &CancellationToken::new(),
            &mut ignore,
        )
        .await
        .unwrap_err();

    assert_eq!(redirect.status, 302);
    assert_eq!(late.code, HttpErrorCode::Timeout);
}

#[tokio::test]
async fn each_collection_keeps_its_own_cookies() {
    let base = support::streaming_server().await;
    let sender = Sender::new(SendOptions::default()).unwrap();
    let mine = Exchange {
        jar: Some("collection-a".into()),
        ..Exchange::default()
    };
    let other = Exchange {
        jar: Some("collection-b".into()),
        ..Exchange::default()
    };
    let none = CancellationToken::new();

    sender
        .send_with(
            &get(format!("{base}/set-cookie")),
            &mine,
            &none,
            &mut ignore,
        )
        .await
        .unwrap();
    let again = sender
        .send_with(&get(format!("{base}/echo")), &mine, &none, &mut ignore)
        .await
        .unwrap();
    let elsewhere = sender
        .send_with(&get(format!("{base}/echo")), &other, &none, &mut ignore)
        .await
        .unwrap();

    assert_eq!(again.body.text.as_deref(), Some("session=abc"));
    assert_eq!(elsewhere.body.text.as_deref(), Some(""));
}

#[tokio::test]
async fn server_sent_events_arrive_one_by_one_and_end_up_in_the_body() {
    let base = support::streaming_server().await;
    let sender = Sender::new(SendOptions::default()).unwrap();
    let mut events: Vec<(StreamEvent, std::time::Instant)> = Vec::new();
    let started = std::time::Instant::now();

    let response = sender
        .send_with(
            &get(format!("{base}/sse")),
            &Exchange::default(),
            &CancellationToken::new(),
            &mut |e| {
                events.push((e, std::time::Instant::now()));
            },
        )
        .await
        .unwrap();

    assert_eq!(
        events
            .iter()
            .map(|(e, _)| e.data.as_str())
            .collect::<Vec<_>>(),
        vec!["one", "two"]
    );
    assert_eq!(events[1].0.event.as_deref(), Some("tick"));
    assert!(
        events[0].1 - started < Duration::from_millis(90),
        "the first event did not wait for the second"
    );
    assert_eq!(
        response.body.text.as_deref(),
        Some("data: one\n\nevent: tick\ndata: two\n\n")
    );
    assert_eq!(response.stream_events, 2);
}

/// Needs the internet: `cargo test -p wbl-http -- --ignored`.
#[tokio::test]
#[ignore = "needs the internet"]
async fn certificate_checks_can_be_turned_off_for_a_request() {
    let sender = Sender::new(SendOptions::default()).unwrap();
    let insecure = Exchange {
        settings: RequestSettings {
            verify_tls: false,
            ..RequestSettings::default()
        },
        ..Exchange::default()
    };
    let response = sender
        .send_with(
            &get("https://self-signed.badssl.com/".into()),
            &insecure,
            &CancellationToken::new(),
            &mut ignore,
        )
        .await
        .unwrap();
    assert_eq!(response.status, 200);
}
