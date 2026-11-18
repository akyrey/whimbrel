#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::time::Duration;

use tokio_util::sync::CancellationToken;
use wbl_core::{Body, BodyKind, Header, Request};
use wbl_http::{HttpErrorCode, SendOptions, Sender};

fn request(method: &str, url: String) -> Request {
    Request {
        method: method.into(),
        url,
        headers: vec![],
        body: Body::None,
    }
}

fn sender() -> Sender {
    Sender::new(SendOptions::default()).unwrap()
}

#[tokio::test]
async fn sends_a_get_and_reads_status_headers_body_and_timings() {
    let base = support::server().await;

    let response = sender()
        .send(
            &request("GET", format!("{base}/echo")),
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.reason.as_deref(), Some("OK"));
    assert_eq!(response.http_version, "HTTP/1.1");
    let traces: Vec<_> = response
        .headers
        .iter()
        .filter(|h| h.name == "x-trace")
        .map(|h| h.value.as_str())
        .collect();
    assert_eq!(
        traces,
        vec!["1", "2"],
        "duplicate headers are kept in order"
    );
    let body: serde_json::Value =
        serde_json::from_str(response.body.text.as_deref().unwrap()).unwrap();
    assert_eq!(body["method"], "GET");
    assert!(response.timing.headers_ms <= response.timing.total_ms);
    assert_eq!(response.body.size, response.body.text.unwrap().len() as u64);
}

#[tokio::test]
async fn sends_a_json_body_with_a_default_content_type_and_the_users_headers() {
    let base = support::server().await;
    let post = Request {
        headers: vec![
            Header {
                name: "X-Api".into(),
                value: "a".into(),
                enabled: true,
            },
            Header {
                name: "X-Off".into(),
                value: "no".into(),
                enabled: false,
            },
        ],
        body: Body::Text {
            kind: BodyKind::Json,
            text: r#"{"name":"ada"}"#.into(),
        },
        ..request("POST", format!("{base}/echo"))
    };

    let response = sender()
        .send(&post, &CancellationToken::new())
        .await
        .unwrap();

    let seen: serde_json::Value = serde_json::from_str(&response.body.text.unwrap()).unwrap();
    let headers = seen["headers"].as_array().unwrap();
    let has = |name: &str, value: &str| {
        headers
            .iter()
            .any(|h| h[0].as_str().unwrap().eq_ignore_ascii_case(name) && h[1] == value)
    };
    assert_eq!(seen["method"], "POST");
    assert_eq!(seen["body"], r#"{"name":"ada"}"#);
    assert!(has("content-type", "application/json"), "{headers:?}");
    assert!(has("x-api", "a"));
    assert!(!has("x-off", "no"), "disabled headers are not sent");
}

#[tokio::test]
async fn sends_custom_methods_such_as_query() {
    let base = support::server().await;

    let response = sender()
        .send(
            &request("QUERY", format!("{base}/echo")),
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    let seen: serde_json::Value = serde_json::from_str(&response.body.text.unwrap()).unwrap();
    assert_eq!(seen["method"], "QUERY");
}

#[tokio::test]
async fn follows_redirects_and_reports_where_it_ended() {
    let base = support::server().await;

    let response = sender()
        .send(
            &request("GET", format!("{base}/redirect")),
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.url, format!("{base}/echo"));
}

#[tokio::test]
async fn an_error_status_is_a_response_not_an_error() {
    let base = support::server().await;

    let response = sender()
        .send(
            &request("GET", format!("{base}/missing")),
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 404);
    assert_eq!(response.body.text.as_deref(), Some("no such route"));
}

#[tokio::test]
async fn stops_reading_at_the_body_cap() {
    let base = support::server().await;
    let small = Sender::new(SendOptions {
        max_body_bytes: 1000,
        ..SendOptions::default()
    })
    .unwrap();

    let response = small
        .send(
            &request("GET", format!("{base}/big")),
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    assert!(response.body.truncated);
    assert_eq!(response.body.size, 1000);
    assert_eq!(response.body.text.unwrap().len(), 1000);
}

#[tokio::test]
async fn a_binary_body_has_no_text() {
    let base = support::server().await;

    let response = sender()
        .send(
            &request("GET", format!("{base}/binary")),
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(response.body.text, None);
    assert_eq!(response.body.size, 4);
    assert_eq!(response.body.content_type.as_deref(), Some("image/png"));
}

#[tokio::test]
async fn cancelling_ends_the_request_with_cancelled() {
    let base = support::server().await;
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        trigger.cancel();
    });

    let error = sender()
        .send(&request("GET", format!("{base}/slow")), &cancel)
        .await
        .unwrap_err();

    assert_eq!(error.code, HttpErrorCode::Cancelled);
}

#[tokio::test]
async fn a_slow_server_times_out() {
    let base = support::server().await;
    let quick = Sender::new(SendOptions {
        timeout: Duration::from_millis(200),
        ..SendOptions::default()
    })
    .unwrap();

    let error = quick
        .send(
            &request("GET", format!("{base}/slow")),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.code, HttpErrorCode::Timeout);
}

#[tokio::test]
async fn nothing_listening_is_a_connect_error() {
    let base = support::closed_port().await;

    let error = sender()
        .send(
            &request("GET", format!("{base}/")),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.code, HttpErrorCode::Connect);
    assert!(
        error.message.starts_with("Could not connect"),
        "{}",
        error.message
    );
}

#[tokio::test]
async fn an_invalid_request_is_refused_before_sending() {
    let error = sender()
        .send(
            &request("GET", "localhost/users".into()),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.code, HttpErrorCode::InvalidRequest);
    assert_eq!(error.field.as_deref(), Some("url"));
    assert_eq!(error.message, "The URL needs http:// or https://");
}

/// Needs the internet: `cargo test -p wbl-http -- --ignored`.
#[tokio::test]
#[ignore = "needs the internet"]
async fn an_untrusted_certificate_is_a_tls_error() {
    let error = sender()
        .send(
            &request("GET", "https://self-signed.badssl.com/".into()),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.code, HttpErrorCode::Tls, "{}", error.message);
}
