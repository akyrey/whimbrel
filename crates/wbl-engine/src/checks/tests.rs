#![allow(clippy::unwrap_used, clippy::expect_used)]

use wbl_core::check::{Assertion, Capture, CaptureFrom};
use wbl_core::interpolate::Value;
use wbl_format::{Environment, SecretName};

use super::*;
use crate::test_support::{TOKEN, auth_server, fixture};

fn login(f: &crate::test_support::Fixture, base: &str, secret: bool) -> RequestSendArgs {
    let plain = f.args(&format!("{base}/login"));
    RequestSendArgs {
        request: wbl_core::Request {
            method: "POST".into(),
            ..plain.request.clone()
        },
        captures: Some(vec![Capture {
            var: "token".into(),
            from: CaptureFrom::Body,
            path: "$.token".into(),
            regex: None,
            secret,
        }]),
        ..plain
    }
}

#[tokio::test]
async fn a_capture_sets_a_runtime_variable_and_assertions_are_evaluated() {
    let f = fixture();
    let base = auth_server().await;
    let root = f.root();

    let logged_in = f.send(login(&f, &base, false)).await.unwrap();
    let me = f
        .send(RequestSendArgs {
            request: wbl_core::Request {
                headers: vec![wbl_core::Header {
                    name: "Authorization".into(),
                    value: "Bearer {{token}}".into(),
                    enabled: true,
                }],
                ..f.args(&format!("{base}/me")).request
            },
            assertions: Some(vec![
                Assertion::Status { equals: 200 },
                Assertion::Status { equals: 201 },
            ]),
            ..f.args("")
        })
        .await
        .unwrap();

    assert_eq!(logged_in.checks.captures[0].value.as_deref(), Some(TOKEN));
    assert!(logged_in.checks.passed());
    assert_eq!(me.response.status, 200);
    assert!(me.checks.assertions[0].passed);
    assert!(!me.checks.passed(), "the second assertion fails");
    let scope = f.runtime.scope(&runtime_key(Some(&root))).unwrap();
    assert_eq!(
        scope
            .entries()
            .find(|(n, _)| *n == "token")
            .map(|(_, v)| v.clone()),
        Some(Value::Plain(TOKEN.into()))
    );
}

#[tokio::test]
async fn a_secret_capture_is_hidden_and_saved_when_the_environment_declares_it() {
    let f = fixture();
    let base = auth_server().await;
    let root = f.root();
    let env = wbl_format::collection::environments::create(
        &root,
        &Environment {
            name: "Dev".into(),
            protected: false,
            variables: Default::default(),
            secrets: vec![SecretName {
                name: "token".into(),
                description: String::new(),
            }],
        },
    )
    .unwrap();

    let declared = f
        .send(RequestSendArgs {
            environment: Some(env.clone()),
            ..login(&f, &base, true)
        })
        .await
        .unwrap();
    let undeclared = f.send(login(&f, &base, true)).await.unwrap();

    let capture = &declared.checks.captures[0];
    assert!(capture.secret && capture.saved && capture.value.is_none());
    assert_eq!(
        f.secrets
            .get(&f.collection_id(), &env, "token")
            .unwrap()
            .as_deref(),
        Some(TOKEN)
    );
    assert!(!undeclared.checks.captures[0].saved);
}

#[tokio::test]
async fn a_capture_that_finds_nothing_reports_why() {
    let f = fixture();
    let base = auth_server().await;
    let args = RequestSendArgs {
        captures: Some(vec![Capture {
            var: "id".into(),
            from: CaptureFrom::Body,
            path: "$.missing".into(),
            regex: None,
            secret: false,
        }]),
        ..login(&f, &base, false)
    };

    let sent = f.send(args).await.unwrap();

    assert!(sent.checks.captures[0].error.is_some());
    assert!(!sent.checks.passed());
}

#[test]
fn select_returns_matches_or_a_stable_code() {
    assert_eq!(
        select(r#"{"a":[1,2]}"#, "$.a[*]").unwrap(),
        "[\n  1,\n  2\n]"
    );
    assert_eq!(select("not json", "$.a").unwrap_err().code, "not_json");
    assert_eq!(select("{}", "$[").unwrap_err().code, "invalid_json_path");
}
