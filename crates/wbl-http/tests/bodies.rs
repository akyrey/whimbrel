#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 8.1: form, multipart and file bodies as they reach the server.

mod support;

use tokio_util::sync::CancellationToken;
use wbl_core::{Body, FormField, Request};
use wbl_http::{SendOptions, Sender};

fn field(name: &str, value: &str, file: bool) -> FormField {
    FormField {
        name: name.into(),
        value: value.into(),
        enabled: true,
        file,
    }
}

async fn seen(body: Body) -> serde_json::Value {
    let base = support::server().await;
    let request = Request {
        method: "POST".into(),
        url: format!("{base}/echo"),
        headers: vec![],
        body,
    };
    let sender = Sender::new(SendOptions::default()).unwrap();
    let response = sender
        .send(&request, &CancellationToken::new())
        .await
        .unwrap();
    serde_json::from_str(response.body.text.as_deref().unwrap()).unwrap()
}

fn header<'a>(seen: &'a serde_json::Value, name: &str) -> Option<&'a str> {
    seen["headers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h[0].as_str().unwrap().eq_ignore_ascii_case(name))
        .and_then(|h| h[1].as_str())
}

#[tokio::test]
async fn a_form_is_url_encoded() {
    let off = FormField {
        enabled: false,
        ..field("skip", "me", false)
    };
    let echo = seen(Body::Form {
        fields: vec![
            field("name", "Ada Lovelace", false),
            field("note", "a&b=c", false),
            off,
        ],
    })
    .await;

    assert_eq!(echo["body"], "name=Ada+Lovelace&note=a%26b%3Dc");
    assert_eq!(
        header(&echo, "content-type"),
        Some("application/x-www-form-urlencoded")
    );
}

#[tokio::test]
async fn multipart_sends_fields_and_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, "file contents").unwrap();
    let path = path.to_string_lossy().into_owned();

    let echo = seen(Body::Multipart {
        fields: vec![field("title", "Hello", false), field("doc", &path, true)],
    })
    .await;

    let body = echo["body"].as_str().unwrap();
    assert!(
        header(&echo, "content-type")
            .unwrap()
            .starts_with("multipart/form-data; boundary=")
    );
    assert!(body.contains("name=\"title\"\r\n\r\nHello"), "{body}");
    assert!(
        body.contains("name=\"doc\"; filename=\"notes.txt\""),
        "{body}"
    );
    assert!(body.contains("file contents"), "{body}");
}

#[tokio::test]
async fn a_file_is_sent_as_it_is_and_a_missing_one_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.bin");
    std::fs::write(&path, "raw bytes").unwrap();

    let echo = seen(Body::File {
        path: path.to_string_lossy().into_owned(),
    })
    .await;

    assert_eq!(echo["body"], "raw bytes");
    assert_eq!(
        header(&echo, "content-type"),
        Some("application/octet-stream")
    );
    let sender = Sender::new(SendOptions::default()).unwrap();
    let missing = Request {
        method: "POST".into(),
        url: "http://127.0.0.1:9/".into(),
        headers: vec![],
        body: Body::File {
            path: "/no/such/file".into(),
        },
    };
    let error = sender
        .send(&missing, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.field.as_deref(), Some("body"));
    assert!(error.message.contains("/no/such/file"));
}
