#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Mutex;

use wbl_core::example::Example;
use wbl_core::{Body, Request};
use wbl_format::StoredRequest;
use wbl_http::{Response, ResponseBody, Timing};

use super::*;
use crate::test_support::fixture;

/// Answers each send from a script, by URL: `fail` gives an error, anything else 200 (with a
/// failing assertion when the URL holds `bad`). Records what it was asked to send.
#[derive(Default)]
struct Fake {
    seen: Mutex<Vec<RequestSendArgs>>,
    cancel_after: Option<(usize, CancellationToken)>,
}

fn ok(status: u16) -> Response {
    Response {
        status,
        reason: None,
        http_version: String::new(),
        url: String::new(),
        headers: vec![],
        body: ResponseBody {
            text: None,
            base64: None,
            size: 0,
            truncated: false,
            file: None,
            content_type: None,
        },
        timing: Timing {
            headers_ms: 1,
            total_ms: 2,
            dns_ms: None,
            connect_ms: None,
        },
        stream_events: 0,
    }
}

impl Dispatch for Fake {
    async fn send(&self, args: RequestSendArgs) -> Result<Sent, EngineError> {
        let count = {
            let mut seen = self.seen.lock().unwrap();
            seen.push(args.clone());
            seen.len()
        };
        if let Some((after, token)) = &self.cancel_after
            && count == *after
        {
            token.cancel();
        }
        if args.request.url.contains("fail") {
            return Err(EngineError::new("connect", "refused"));
        }
        let checks = Checks {
            assertions: if args.request.url.contains("bad") {
                vec![wbl_core::check::CheckResult {
                    passed: false,
                    message: "status 200, expected 201".into(),
                }]
            } else {
                vec![]
            },
            captures: vec![],
        };
        Ok(Sent {
            response: ok(200),
            checks,
        })
    }
}

fn stored(name: &str, url: &str, examples: Vec<Example>) -> StoredRequest {
    StoredRequest {
        name: name.into(),
        order: 0,
        request: Request {
            method: "GET".into(),
            url: url.into(),
            headers: vec![],
            body: Body::None,
        },
        examples,
        settings: Default::default(),
        auth: Default::default(),
        variables: Default::default(),
        captures: vec![],
        assertions: vec![],
    }
}

fn example(id: &str) -> Example {
    Example::named(id, &format!("Example {id}"))
}

fn start(f: &crate::test_support::Fixture, steps: Vec<RunStep>, stop: bool) -> RunStartArgs {
    RunStartArgs {
        run_id: "r1".into(),
        root: f.root_str(),
        environment: Some("dev".into()),
        steps,
        stop_on_failure: stop,
    }
}

fn step_of(path: &str, mode: StepMode) -> RunStep {
    RunStep {
        path: path.into(),
        mode,
    }
}

async fn go(
    fake: &Fake,
    f: &crate::test_support::Fixture,
    args: &RunStartArgs,
    token: &CancellationToken,
) -> (RunSummary, Vec<StepResult>) {
    let mut results = Vec::new();
    let summary = run(fake, &f.root(), args, token, &mut |r| results.push(r))
        .await
        .unwrap();
    (summary, results)
}

#[tokio::test]
async fn steps_expand_per_example_and_pass_the_environment() {
    let f = fixture();
    let root = f.root();
    let a = wbl_format::collection::create_request(&root, "", &stored("A", "http://a", vec![]))
        .unwrap();
    let b = wbl_format::collection::create_request(
        &root,
        "",
        &stored("B", "http://b", vec![example("x"), example("y")]),
    )
    .unwrap();
    let fake = Fake::default();
    let args = start(
        &f,
        vec![
            step_of(&a, StepMode::EachExample),
            step_of(&b, StepMode::EachExample),
            step_of(&b, StepMode::Example { id: "y".into() }),
            step_of(&b, StepMode::Base),
        ],
        false,
    );

    let (summary, results) = go(&fake, &f, &args, &CancellationToken::new()).await;

    assert_eq!(summary.passed, 5);
    let examples: Vec<Option<&str>> = results.iter().map(|r| r.example.as_deref()).collect();
    assert_eq!(
        examples,
        [
            None,
            Some("Example x"),
            Some("Example y"),
            Some("Example y"),
            None
        ]
    );
    let seen = fake.seen.lock().unwrap();
    assert!(seen.iter().all(|s| s.environment.as_deref() == Some("dev")));
    assert!(seen.iter().all(|s| s.request_id == request_id("r1")));
    assert_eq!(seen[1].path.as_deref(), Some(b.as_str()));
}

#[tokio::test]
async fn a_failure_stops_the_run_when_asked_and_is_reported() {
    let f = fixture();
    let root = f.root();
    let bad =
        wbl_format::collection::create_request(&root, "", &stored("Bad", "http://bad", vec![]))
            .unwrap();
    let down =
        wbl_format::collection::create_request(&root, "", &stored("Down", "http://fail", vec![]))
            .unwrap();
    let good =
        wbl_format::collection::create_request(&root, "", &stored("Good", "http://good", vec![]))
            .unwrap();
    let steps = || {
        vec![
            step_of(&down, StepMode::Base),
            step_of(&bad, StepMode::Base),
            step_of(&good, StepMode::Base),
        ]
    };

    let (all, results) = go(
        &Fake::default(),
        &f,
        &start(&f, steps(), false),
        &CancellationToken::new(),
    )
    .await;
    let (stopped, _) = go(
        &Fake::default(),
        &f,
        &start(&f, steps(), true),
        &CancellationToken::new(),
    )
    .await;

    assert_eq!((all.passed, all.failed, all.skipped), (1, 2, 0));
    assert_eq!(results[0].error.as_deref(), Some("refused"));
    assert_eq!(results[1].status, Some(200));
    assert!(!results[1].passed);
    assert_eq!((stopped.passed, stopped.failed, stopped.skipped), (0, 1, 2));
}

#[tokio::test]
async fn cancelling_skips_what_is_left() {
    let f = fixture();
    let root = f.root();
    let a = wbl_format::collection::create_request(&root, "", &stored("A", "http://a", vec![]))
        .unwrap();
    let token = CancellationToken::new();
    let fake = Fake {
        cancel_after: Some((1, token.clone())),
        ..Fake::default()
    };
    let steps = vec![step_of(&a, StepMode::Base); 3];

    let (summary, _) = go(&fake, &f, &start(&f, steps, false), &token).await;

    assert!(summary.cancelled);
    assert_eq!((summary.passed, summary.skipped), (1, 2));
}

#[tokio::test]
async fn an_unknown_example_fails_before_sending() {
    let f = fixture();
    let root = f.root();
    let a = wbl_format::collection::create_request(&root, "", &stored("A", "http://a", vec![]))
        .unwrap();
    let fake = Fake::default();
    let args = start(
        &f,
        vec![step_of(&a, StepMode::Example { id: "nope".into() })],
        false,
    );

    let error = run(&fake, &root, &args, &CancellationToken::new(), &mut |_| {})
        .await
        .unwrap_err();

    assert_eq!(error.code, "not_found");
    assert!(fake.seen.lock().unwrap().is_empty());
}
