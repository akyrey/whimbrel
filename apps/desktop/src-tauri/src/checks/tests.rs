#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 7.2: captures feed runtime variables, assertions are evaluated, secret captures are
//! masked and saved to the keychain when the environment declares them.

use std::sync::Arc;

use wbl_core::check::{Assertion, Capture, CaptureFrom};
use wbl_core::{Body, Header, Request};
use wbl_format::{Environment, SecretName};
use wbl_http::SendOptions;

use super::*;
use crate::commands::{preview, send};
use crate::ipc::CollectionCreateArgs;
use crate::runtime::{RuntimeArgs, clear, list};
use crate::state::AppState;

use crate::test_support::{TOKEN, auth_server as server};

struct Fixture {
    _dir: tempfile::TempDir,
    root: String,
    state: AppState,
    collections: Collections,
    secrets: Secrets,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let collections = Collections::default();
    collections
        .create(&CollectionCreateArgs {
            root: root.clone(),
            name: "API".into(),
        })
        .unwrap();
    Fixture {
        _dir: dir,
        root,
        state: AppState::new(SendOptions::default()).unwrap(),
        collections,
        secrets: Secrets::new(Arc::new(roost_keychain::MemorySecretStore::default())),
    }
}

fn args(f: &Fixture, method: &str, url: String, environment: Option<&str>) -> RequestSendArgs {
    RequestSendArgs {
        request_id: "tab".into(),
        request: Request {
            method: method.into(),
            url,
            headers: vec![Header {
                name: "Authorization".into(),
                value: "Bearer {{token}}".into(),
                enabled: true,
            }],
            body: Body::None,
        },
        root: Some(f.root.clone()),
        environment: environment.map(str::to_owned),
        example: None,
        settings: None,
        name: None,
        path: None,
        auth: None,
        variables: None,
        captures: None,
        assertions: None,
    }
}

fn login(f: &Fixture, base: &str, secret: bool, environment: Option<&str>) -> RequestSendArgs {
    let plain = args(f, "POST", format!("{base}/login"), environment);
    RequestSendArgs {
        request: Request {
            headers: vec![],
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

fn me(f: &Fixture, base: &str, environment: Option<&str>) -> RequestSendArgs {
    RequestSendArgs {
        assertions: Some(vec![
            Assertion::Status { equals: 200 },
            Assertion::Json {
                path: "$.name".into(),
                equals: Some(serde_json::json!("ada")),
            },
        ]),
        ..args(f, "GET", format!("{base}/me"), environment)
    }
}

#[tokio::test]
async fn a_captured_token_authorises_the_next_request() {
    let f = fixture();
    let base = server().await;

    let logged_in = send(
        &f.state,
        &f.collections,
        &f.secrets,
        &mut |_| {},
        login(&f, &base, false, None),
    )
    .await
    .unwrap();
    let answer = send(
        &f.state,
        &f.collections,
        &f.secrets,
        &mut |_| {},
        me(&f, &base, None),
    )
    .await
    .unwrap();

    assert_eq!(logged_in.checks.captures[0].value.as_deref(), Some(TOKEN));
    assert!(logged_in.checks.passed());
    assert_eq!(answer.response.status, 200);
    assert!(answer.checks.passed(), "{:?}", answer.checks);
    let runtime = list(
        &f.collections,
        &RuntimeArgs {
            root: Some(f.root.clone()),
        },
    )
    .unwrap();
    assert_eq!(runtime[0].value.as_deref(), Some(TOKEN));

    clear(
        &f.collections,
        &RuntimeArgs {
            root: Some(f.root.clone()),
        },
    )
    .unwrap();
    let missing = preview(&f.collections, &f.secrets, &me(&f, &base, None)).unwrap_err();
    assert_eq!(missing.code, "unresolved");
}

#[tokio::test]
async fn failing_checks_are_reported() {
    let f = fixture();
    let base = server().await;
    let bad_capture = RequestSendArgs {
        captures: Some(vec![Capture {
            var: "x".into(),
            from: CaptureFrom::Body,
            path: "$.nope".into(),
            regex: None,
            secret: false,
        }]),
        ..login(&f, &base, false, None)
    };

    let captured = send(
        &f.state,
        &f.collections,
        &f.secrets,
        &mut |_| {},
        bad_capture,
    )
    .await
    .unwrap();
    let unauthorised = RequestSendArgs {
        variables: Some([("token".to_owned(), "wrong".to_owned())].into()),
        ..me(&f, &base, None)
    };
    let refused = send(
        &f.state,
        &f.collections,
        &f.secrets,
        &mut |_| {},
        unauthorised,
    )
    .await
    .unwrap();

    assert_eq!(
        captured.checks.captures[0].error.as_deref(),
        Some("Nothing found at $.nope")
    );
    assert!(!captured.checks.passed());
    assert_eq!(
        refused.checks.assertions[0].message,
        "status 401, expected 200"
    );
    assert!(!refused.checks.passed());
}

#[tokio::test]
async fn a_secret_capture_is_masked_and_saved_when_the_environment_declares_it() {
    let f = fixture();
    let base = server().await;
    let root = std::path::Path::new(&f.root);
    let env = wbl_format::collection::environments::create(
        root,
        &Environment {
            name: "dev".into(),
            protected: false,
            variables: Default::default(),
            secrets: vec![SecretName {
                name: "token".into(),
                description: String::new(),
            }],
        },
    )
    .unwrap();

    let logged_in = send(
        &f.state,
        &f.collections,
        &f.secrets,
        &mut |_| {},
        login(&f, &base, true, Some(&env)),
    )
    .await
    .unwrap();
    let shown = preview(&f.collections, &f.secrets, &me(&f, &base, Some(&env))).unwrap();
    let answer = send(
        &f.state,
        &f.collections,
        &f.secrets,
        &mut |_| {},
        me(&f, &base, Some(&env)),
    )
    .await
    .unwrap();

    let result = &logged_in.checks.captures[0];
    assert!(result.secret && result.saved && result.value.is_none());
    assert_eq!(
        shown.headers[0].value,
        format!("Bearer {}", wbl_core::interpolate::MASK)
    );
    assert!(answer.checks.passed());
    let id = wbl_format::collection::id(root).unwrap();
    assert_eq!(
        f.secrets.get(&id, &env, "token").unwrap().as_deref(),
        Some(TOKEN)
    );
    let runtime = list(
        &f.collections,
        &RuntimeArgs {
            root: Some(f.root.clone()),
        },
    )
    .unwrap();
    assert!(runtime[0].secret && runtime[0].value.is_none());

    let loose = send(
        &f.state,
        &f.collections,
        &f.secrets,
        &mut |_| {},
        login(&f, &base, true, None),
    )
    .await
    .unwrap();
    assert!(
        !loose.checks.captures[0].saved,
        "no environment: kept in memory only"
    );
}

/// Task 9.4: the response view's JSONPath filter.
#[test]
fn selects_matches_as_pretty_json_and_names_the_errors() {
    let args = |path: &str| crate::ipc::ResponseSelectArgs {
        body: r#"{"users":[{"name":"ada"},{"name":"grace"}]}"#.into(),
        path: path.into(),
    };
    assert_eq!(
        select(&args("$.users[*].name")).unwrap(),
        "[\n  \"ada\",\n  \"grace\"\n]"
    );
    assert_eq!(
        select(&args("$.users[")).unwrap_err().code,
        "invalid_json_path"
    );
    let not_json = crate::ipc::ResponseSelectArgs {
        body: "<html>".into(),
        path: "$".into(),
    };
    assert_eq!(select(&not_json).unwrap_err().code, "not_json");
}
