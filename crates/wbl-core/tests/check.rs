#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 7.1: captures and assertions against a received response.

use serde_json::json;
use wbl_core::check::{Assertion, Capture, CaptureFrom, Observed, assert, capture};

fn observed() -> Observed {
    Observed {
        status: 201,
        headers: vec![
            ("Content-Type".into(), "application/json".into()),
            ("Location".into(), "/users/42".into()),
        ],
        body: Some(
            r#"{"token":"abc.def","user":{"id":42,"roles":["admin","dev"]},"ok":true}"#.into(),
        ),
        total_ms: 120,
    }
}

fn body(path: &str) -> Capture {
    Capture {
        var: "v".into(),
        from: CaptureFrom::Body,
        path: path.into(),
        regex: None,
        secret: false,
    }
}

#[test]
fn captures_from_the_body_headers_and_regexes() {
    let o = observed();
    assert_eq!(capture(&body("$.token"), &o).unwrap(), "abc.def");
    assert_eq!(capture(&body("$.user.id"), &o).unwrap(), "42");
    assert_eq!(capture(&body("$.user.roles[1]"), &o).unwrap(), "dev");
    assert_eq!(
        capture(&body("$.user.roles"), &o).unwrap(),
        r#"["admin","dev"]"#
    );
    assert_eq!(capture(&body(""), &o).unwrap(), o.body.clone().unwrap());
    let location = Capture {
        from: CaptureFrom::Header,
        path: "location".into(),
        regex: Some(r"/users/(\d+)".into()),
        ..body("")
    };
    assert_eq!(capture(&location, &o).unwrap(), "42");
    let whole = Capture {
        regex: Some(r"\w+".into()),
        ..body("$.token")
    };
    assert_eq!(capture(&whole, &o).unwrap(), "abc");
}

#[test]
fn capture_failures_say_why() {
    let o = observed();
    assert_eq!(
        capture(&body("$.nope"), &o).unwrap_err().to_string(),
        "Nothing found at $.nope"
    );
    assert!(
        capture(&body("$["), &o)
            .unwrap_err()
            .to_string()
            .contains("not a JSONPath")
    );
    let bad = Capture {
        regex: Some("(".into()),
        ..body("$.token")
    };
    assert!(
        capture(&bad, &o)
            .unwrap_err()
            .to_string()
            .contains("not a regular expression")
    );
    let unmatched = Capture {
        regex: Some("zzz".into()),
        ..body("$.token")
    };
    assert!(capture(&unmatched, &o).is_err());
    let text = Observed {
        body: Some("plain".into()),
        ..o.clone()
    };
    assert_eq!(
        capture(&body("$.a"), &text).unwrap_err().to_string(),
        "The body is not JSON"
    );
    let header = Capture {
        from: CaptureFrom::Header,
        path: "X-None".into(),
        ..body("")
    };
    assert!(capture(&header, &o).is_err());
}

#[test]
fn assertions_pass_and_fail_with_messages() {
    let o = observed();
    let check = |a: Assertion| assert(&a, &o);

    assert!(check(Assertion::Status { equals: 201 }).passed);
    assert_eq!(
        check(Assertion::Status { equals: 200 }).message,
        "status 201, expected 200"
    );
    assert!(
        check(Assertion::Header {
            name: "content-type".into(),
            equals: None
        })
        .passed
    );
    assert!(
        check(Assertion::Header {
            name: "Location".into(),
            equals: Some("/users/42".into())
        })
        .passed
    );
    assert!(
        !check(Assertion::Header {
            name: "Location".into(),
            equals: Some("/x".into())
        })
        .passed
    );
    assert!(
        !check(Assertion::Header {
            name: "X-Missing".into(),
            equals: None
        })
        .passed
    );
    assert!(
        check(Assertion::Json {
            path: "$.user.id".into(),
            equals: Some(json!(42))
        })
        .passed
    );
    assert!(
        check(Assertion::Json {
            path: "$.ok".into(),
            equals: None
        })
        .passed
    );
    let wrong = check(Assertion::Json {
        path: "$.user.id".into(),
        equals: Some(json!("42")),
    });
    assert_eq!(wrong.message, r#"$.user.id is 42, expected "42""#);
    assert!(
        !check(Assertion::Json {
            path: "$.gone".into(),
            equals: None
        })
        .passed
    );
    assert!(
        !check(Assertion::Json {
            path: "$[".into(),
            equals: None
        })
        .passed
    );
    assert!(check(Assertion::Time { below_ms: 500 }).passed);
    assert_eq!(
        check(Assertion::Time { below_ms: 100 }).message,
        "120 ms, expected under 100 ms"
    );
}

#[test]
fn assertions_read_from_their_json_form() {
    let a: Assertion =
        serde_json::from_value(json!({"check": "json", "path": "$.a", "equals": [1, 2]})).unwrap();
    assert_eq!(
        a,
        Assertion::Json {
            path: "$.a".into(),
            equals: Some(json!([1, 2]))
        }
    );
    let t: Assertion = serde_json::from_value(json!({"check": "time", "belowMs": 9})).unwrap();
    assert_eq!(t, Assertion::Time { below_ms: 9 });
}
