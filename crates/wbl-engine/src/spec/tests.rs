#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../wbl-openapi/tests/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn sender() -> Sender {
    Sender::new(wbl_http::SendOptions::default()).unwrap()
}

/// Serves `SPEC` at `/spec.yaml`, 404 elsewhere.
async fn server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            let head = String::from_utf8_lossy(&buf[..n]).into_owned();
            let answer = if head.starts_with("GET /spec.yaml ") {
                format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{SPEC}",
                    SPEC.len()
                )
            } else {
                "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".into()
            };
            let _ = socket.write_all(answer.as_bytes()).await;
        }
    });
    base
}

const SPEC: &str = "openapi: 3.1.0\ninfo: {title: Remote, version: '1'}\npaths:\n  /pets:\n    get:\n      operationId: listPets\n      responses: {'200': {description: ok}}\n";

#[tokio::test]
async fn a_file_is_read_and_summarised() {
    let read = read(
        &sender(),
        &SpecSource::File {
            path: fixture("openapi30.yaml"),
        },
        false,
    )
    .await
    .unwrap();

    let summary = summary(&read.api, read.remote);

    assert_eq!(summary.title, "Users");
    assert_eq!(summary.operations, 3);
    assert_eq!(summary.folders, ["users"]);
    assert!(read.source.location.ends_with("openapi30.yaml"));
}

#[tokio::test]
async fn a_url_is_fetched_and_failures_have_codes() {
    let base = server().await;
    let fetched = read(
        &sender(),
        &SpecSource::Url {
            url: format!("{base}/spec.yaml"),
        },
        false,
    )
    .await
    .unwrap();
    let missing = read(
        &sender(),
        &SpecSource::Url {
            url: format!("{base}/nope.yaml"),
        },
        false,
    )
    .await
    .unwrap_err();
    let not_http = read(
        &sender(),
        &SpecSource::Url {
            url: "ftp://x".into(),
        },
        false,
    )
    .await
    .unwrap_err();
    let no_file = read(
        &sender(),
        &SpecSource::File {
            path: "/no/such/spec.yaml".into(),
        },
        false,
    )
    .await
    .unwrap_err();

    assert_eq!(fetched.api.title, "Remote");
    assert!(missing.message.contains("404"));
    assert_eq!(not_http.code, "invalid_url");
    assert_eq!(no_file.code, "not_found");
}

#[tokio::test]
async fn a_kept_spec_is_opened_from_the_collection() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    wbl_format::collection::create(&root, "API").unwrap();
    let read = read(
        &sender(),
        &SpecSource::File {
            path: fixture("openapi30.yaml"),
        },
        false,
    )
    .await
    .unwrap();
    let generated = wbl_openapi::generate(&read.api);
    let installed = wbl_openapi::install(&root, "", &generated, &read.source).unwrap();
    let dir_name = wbl_openapi::specs(&root).unwrap()[0].dir.clone();

    let api = open(&sender(), &root, &dir_name).await.unwrap();
    let gone = open(&sender(), &root, "openapi/none").await.unwrap_err();

    assert!(!installed.requests.is_empty());
    assert_eq!(api.title, "Users");
    assert_eq!(gone.code, "not_found");
}
