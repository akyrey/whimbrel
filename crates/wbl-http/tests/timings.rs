#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 10.1: DNS and connect times for a new connection, none for a reused one (D-H20).

mod support;

use tokio_util::sync::CancellationToken;
use wbl_core::{Body, Request};
use wbl_http::{SendOptions, Sender, Timing};

fn get(url: String) -> Request {
    Request {
        method: "GET".into(),
        url,
        headers: vec![],
        body: Body::None,
    }
}

#[tokio::test]
async fn a_new_connection_has_dns_and_connect_times_and_a_reused_one_has_neither() {
    let base = support::keep_alive_server().await;
    let sender = Sender::new(SendOptions::default()).unwrap();
    let cancel = CancellationToken::new();

    let first = sender
        .send(&get(format!("{base}/echo")), &cancel)
        .await
        .unwrap();
    let dns = first.timing.dns_ms.expect("a name was resolved");
    let connect = first.timing.connect_ms.expect("a connection was made");
    assert!(dns + connect <= first.timing.headers_ms);

    let second = sender
        .send(&get(format!("{base}/echo")), &cancel)
        .await
        .unwrap();
    assert_eq!(second.timing.dns_ms, None);
    assert_eq!(second.timing.connect_ms, None);
}

#[tokio::test]
async fn an_ip_address_needs_no_dns() {
    let base = support::server().await;
    let sender = Sender::new(SendOptions::default()).unwrap();
    let response = sender
        .send(&get(format!("{base}/echo")), &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(response.timing.dns_ms, None);
    assert!(response.timing.connect_ms.is_some());
}

#[test]
fn timings_saved_before_the_breakdown_still_read() {
    let old: Timing = serde_json::from_str(r#"{"headersMs":12,"totalMs":30}"#).unwrap();
    assert_eq!(old.headers_ms, 12);
    assert_eq!(old.dns_ms, None);
    assert_eq!(old.connect_ms, None);
}
