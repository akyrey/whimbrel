#![allow(clippy::unwrap_used, clippy::expect_used)]

use wbl_core::check::{Assertion, Capture, CaptureFrom};
use wbl_core::{Body, BodyKind, Header};
use wbl_format::collection::environments;
use wbl_format::{Environment, SecretName};

use super::*;
use crate::test_support::{Fixture, TOKEN, auth_server, echo_server, fixture};

const SENTINEL: &str = "sentinel-7c1f9a";

fn clients(f: &Fixture) -> Clients<'_> {
    Clients {
        sender: &f.sender,
        oauth: &f.oauth,
    }
}

/// `Dev` declares `key` (holding the sentinel) and `token` as secrets.
fn dev(f: &Fixture) -> String {
    let root = f.root();
    let id = environments::create(
        &root,
        &Environment {
            name: "Dev".into(),
            protected: false,
            variables: Default::default(),
            secrets: ["key", "token"]
                .map(|n| SecretName {
                    name: n.into(),
                    description: String::new(),
                })
                .to_vec(),
        },
    )
    .unwrap();
    f.secrets
        .set(&f.collection_id(), &id, "key", SENTINEL)
        .unwrap();
    id
}

#[tokio::test]
async fn an_echoed_secret_is_masked_in_the_request_response_and_checks() {
    let f = fixture();
    let root = f.root();
    let base = echo_server().await;
    let env = dev(&f);
    let args = RequestSendArgs {
        environment: Some(env),
        request: Request {
            method: "POST".into(),
            headers: vec![Header {
                name: "X-Key".into(),
                value: "{{key}}".into(),
                enabled: true,
            }],
            body: Body::Text {
                kind: BodyKind::Text,
                text: format!("key={{{{key}}}} q=?{SENTINEL}"),
            },
            ..f.args(&format!("{base}/echo?k={{{{key}}}}")).request
        },
        captures: Some(vec![Capture {
            var: "echoed".into(),
            from: CaptureFrom::Body,
            path: String::new(),
            regex: Some("key=(\\S+)".into()),
            secret: false,
        }]),
        assertions: Some(vec![Assertion::Status { equals: 201 }]),
        ..f.args("")
    };

    let delivered = send_once(
        clients(&f),
        &f.sources(&root),
        &args,
        &CancellationToken::new(),
    )
    .await
    .unwrap();

    let all = serde_json::to_string(&delivered).unwrap();
    assert!(!all.contains(SENTINEL), "{all}");
    assert!(delivered.response.body.text.unwrap().contains("••••••"));
    assert_eq!(delivered.response.status, 200);
}

#[tokio::test]
async fn a_secret_just_captured_is_masked_too() {
    let f = fixture();
    let root = f.root();
    let base = auth_server().await;
    let env = dev(&f);
    let args = RequestSendArgs {
        environment: Some(env.clone()),
        request: Request {
            method: "POST".into(),
            ..f.args(&format!("{base}/login")).request
        },
        captures: Some(vec![Capture {
            var: "token".into(),
            from: CaptureFrom::Body,
            path: "$.token".into(),
            regex: None,
            secret: true,
        }]),
        ..f.args("")
    };

    let delivered = send_once(
        clients(&f),
        &f.sources(&root),
        &args,
        &CancellationToken::new(),
    )
    .await
    .unwrap();

    let all = serde_json::to_string(&delivered).unwrap();
    assert!(!all.contains(TOKEN), "{all}");
    assert!(delivered.checks.captures[0].saved);
    assert_eq!(
        f.secrets
            .get(&f.collection_id(), &env, "token")
            .unwrap()
            .as_deref(),
        Some(TOKEN)
    );
}

#[tokio::test]
async fn a_failed_send_reports_its_code() {
    let f = fixture();
    let root = f.root();

    let error = send_once(
        clients(&f),
        &f.sources(&root),
        &f.args("http://127.0.0.1:1/"),
        &CancellationToken::new(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.code, "connect");
}
