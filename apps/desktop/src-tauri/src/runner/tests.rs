#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 7.3 and the Phase 7 acceptance: login → capture token → authorised call, as a run.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use wbl_core::check::{Assertion, Capture, CaptureFrom};
use wbl_core::example::Example;
use wbl_core::{Body, Header, Request};
use wbl_format::StoredRequest;
use wbl_http::SendOptions;

use super::*;
use crate::ipc::CollectionCreateArgs;
use crate::test_support::{auth_server, silent_server};

struct Fixture {
    _dir: tempfile::TempDir,
    root: String,
    state: AppState,
    runs: Runs,
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
        runs: Runs::default(),
        collections,
        secrets: Secrets::new(Arc::new(roost_keychain::MemorySecretStore::default())),
    }
}

fn stored(name: &str, method: &str, url: String) -> StoredRequest {
    StoredRequest {
        name: name.into(),
        order: 0,
        request: Request {
            method: method.into(),
            url,
            headers: vec![],
            body: Body::None,
        },
        examples: vec![],
        settings: Default::default(),
        auth: Default::default(),
        variables: Default::default(),
        captures: vec![],
        assertions: vec![],
    }
}

/// `login.req.toml` captures the token; `me.req.toml` sends it and checks the answer.
fn write_requests(f: &Fixture, base: &str) {
    let root = Path::new(&f.root);
    let login = StoredRequest {
        captures: vec![Capture {
            var: "token".into(),
            from: CaptureFrom::Body,
            path: "$.token".into(),
            regex: None,
            secret: true,
        }],
        ..stored("Login", "POST", format!("{base}/login"))
    };
    let me = StoredRequest {
        request: Request {
            headers: vec![Header {
                name: "Authorization".into(),
                value: "Bearer {{token}}".into(),
                enabled: true,
            }],
            ..stored("Me", "GET", format!("{base}/me")).request
        },
        assertions: vec![Assertion::Status { equals: 200 }],
        examples: vec![
            Example::named("good", "Good"),
            Example {
                variables: [("token".to_owned(), "forged".to_owned())].into(),
                ..Example::named("forged", "Forged")
            },
        ],
        ..stored("Me", "GET", format!("{base}/me"))
    };
    wbl_format::collection::create_request(root, "", &login).unwrap();
    wbl_format::collection::create_request(root, "", &me).unwrap();
}

fn args(f: &Fixture, steps: Vec<RunStep>, stop_on_failure: bool) -> RunStartArgs {
    RunStartArgs {
        run_id: "r1".into(),
        root: f.root.clone(),
        environment: None,
        steps,
        stop_on_failure,
    }
}

fn step(path: &str, mode: StepMode) -> RunStep {
    RunStep {
        path: path.into(),
        mode,
    }
}

async fn run_all(f: &Fixture, args: &RunStartArgs) -> (RunSummary, Vec<StepResult>) {
    let mut seen = Vec::new();
    let summary = run(
        &f.state,
        &f.runs,
        &f.collections,
        &f.secrets,
        args,
        &mut |r| seen.push(r),
    )
    .await
    .unwrap();
    (summary, seen)
}

#[tokio::test]
async fn login_then_an_authorised_call_passes_as_a_run() {
    let f = fixture();
    write_requests(&f, &auth_server().await);

    let (summary, steps) = run_all(
        &f,
        &args(
            &f,
            vec![
                step("login.req.toml", StepMode::Base),
                step("me.req.toml", StepMode::Base),
            ],
            true,
        ),
    )
    .await;

    assert_eq!(
        summary,
        RunSummary {
            passed: 2,
            failed: 0,
            skipped: 0,
            cancelled: false
        }
    );
    assert_eq!(steps[1].status, Some(200));
    assert_eq!(steps[1].name, "Me");
    assert!(steps[0].checks.as_ref().unwrap().captures[0].secret);
}

#[tokio::test]
async fn a_failure_stops_the_run_or_not() {
    let f = fixture();
    write_requests(&f, &auth_server().await);
    let steps = || {
        vec![
            step("me.req.toml", StepMode::Base),
            step("login.req.toml", StepMode::Base),
        ]
    };

    let (stopped, results) = run_all(&f, &args(&f, steps(), true)).await;
    let (went_on, _) = run_all(&f, &args(&f, steps(), false)).await;

    assert_eq!(
        stopped,
        RunSummary {
            passed: 0,
            failed: 1,
            skipped: 1,
            cancelled: false
        }
    );
    assert_eq!(
        results[0].error.as_deref().map(|e| e.contains("token")),
        Some(true),
        "{results:?}"
    );
    assert_eq!((went_on.passed, went_on.failed), (1, 1));
}

#[tokio::test]
async fn a_step_runs_once_per_example_or_with_one() {
    let f = fixture();
    write_requests(&f, &auth_server().await);

    let (summary, results) = run_all(
        &f,
        &args(
            &f,
            vec![
                step("login.req.toml", StepMode::Base),
                step("me.req.toml", StepMode::EachExample),
            ],
            false,
        ),
    )
    .await;
    let (one, single) = run_all(
        &f,
        &args(
            &f,
            vec![step(
                "me.req.toml",
                StepMode::Example {
                    id: "forged".into(),
                },
            )],
            false,
        ),
    )
    .await;

    let examples: Vec<_> = results
        .iter()
        .map(|r| (r.example.clone(), r.passed))
        .collect();
    assert_eq!(
        examples,
        [
            (None, true),
            (Some("Good".into()), true),
            (Some("Forged".into()), false)
        ]
    );
    assert_eq!(summary.failed, 1);
    assert_eq!((one.failed, single[0].status), (1, Some(401)));
    let missing = run(
        &f.state,
        &f.runs,
        &f.collections,
        &f.secrets,
        &args(
            &f,
            vec![step("me.req.toml", StepMode::Example { id: "x".into() })],
            false,
        ),
        &mut |_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code, "not_found");
}

#[tokio::test]
async fn cancelling_stops_the_run() {
    let f = Arc::new(fixture());
    let silent = silent_server().await;
    let root = Path::new(&f.root);
    for name in ["One", "Two"] {
        wbl_format::collection::create_request(
            root,
            "",
            &stored(name, "GET", format!("{silent}/")),
        )
        .unwrap();
    }
    let running = tokio::spawn({
        let f = Arc::clone(&f);
        async move {
            let a = args(
                &f,
                vec![
                    step("one.req.toml", StepMode::Base),
                    step("two.req.toml", StepMode::Base),
                ],
                false,
            );
            run(
                &f.state,
                &f.runs,
                &f.collections,
                &f.secrets,
                &a,
                &mut |_| {},
            )
            .await
        }
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert!(
        cancel(
            &f.state,
            &f.runs,
            &RunCancelArgs {
                run_id: "r1".into()
            }
        )
        .unwrap()
    );
    let summary = running.await.unwrap().unwrap();

    assert_eq!(
        summary,
        RunSummary {
            passed: 0,
            failed: 1,
            skipped: 1,
            cancelled: true
        }
    );
    assert!(
        !cancel(
            &f.state,
            &f.runs,
            &RunCancelArgs {
                run_id: "r1".into()
            }
        )
        .unwrap()
    );
}
