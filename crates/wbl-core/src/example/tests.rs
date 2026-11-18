#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::{Body, BodyKind, Header, Request};

fn header(name: &str, value: &str) -> Header {
    Header {
        name: name.into(),
        value: value.into(),
        enabled: true,
    }
}

fn base() -> Request {
    Request {
        method: "POST".into(),
        url: "{{base}}/users?page=1&notify=false#top".into(),
        headers: vec![
            header("Accept", "application/json"),
            header("X-Tenant", "a"),
        ],
        body: Body::Text {
            kind: BodyKind::Json,
            text: "{}".into(),
        },
    }
}

fn param(name: &str, value: &str, enabled: bool) -> Param {
    Param {
        name: name.into(),
        value: value.into(),
        enabled,
    }
}

#[test]
fn an_example_overlays_query_headers_and_body_without_changing_the_base() {
    let example = Example {
        query: vec![
            param("notify", "true", true),
            param("page", "", false),
            param("q", "ada", true),
        ],
        headers: vec![
            Header {
                enabled: true,
                ..header("x-tenant", "b")
            },
            header("X-New", "1"),
        ],
        body: Some(Body::Text {
            kind: BodyKind::Json,
            text: r#"{"a":1}"#.into(),
        }),
        ..Example::named("e1", "Example A")
    };
    let original = base();

    let applied = apply(&original, &example);

    assert_eq!(applied.url, "{{base}}/users?notify=true&q=ada#top");
    assert_eq!(
        applied.headers,
        vec![
            header("Accept", "application/json"),
            header("X-Tenant", "b"),
            header("X-New", "1")
        ]
    );
    assert_eq!(
        applied.body,
        Body::Text {
            kind: BodyKind::Json,
            text: r#"{"a":1}"#.into()
        }
    );
    assert_eq!(original, base());
}

#[test]
fn a_disabled_header_in_an_example_turns_the_base_one_off() {
    let example = Example {
        headers: vec![Header {
            enabled: false,
            ..header("accept", "")
        }],
        ..Example::named("e", "E")
    };

    let applied = apply(&base(), &example);

    assert!(!applied.headers[0].enabled);
    assert_eq!(
        applied.headers[0].value, "application/json",
        "the value stays"
    );
}

#[test]
fn an_empty_example_changes_nothing_and_its_variables_become_a_scope() {
    let example = Example {
        variables: [("role".to_owned(), "admin".to_owned())].into(),
        ..Example::named("e", "E")
    };

    assert_eq!(apply(&base(), &example), base());
    assert_eq!(
        example.scope(),
        Scope::new([("role".to_owned(), Value::Plain("admin".into()))].into())
    );
}

#[test]
fn a_url_without_a_query_gets_one() {
    let mut plain = base();
    plain.url = "http://x/a".into();
    let example = Example {
        query: vec![param("q", "1", true)],
        ..Example::named("e", "E")
    };
    assert_eq!(apply(&plain, &example).url, "http://x/a?q=1");
}

#[test]
fn the_overlay_of_an_edit_is_what_changed() {
    let mut draft = base();
    draft.url = "{{base}}/users?page=2&notify=false&extra=1#top".into();
    draft.headers[1].value = "b".into();
    draft.headers.push(header("X-Debug", "1"));
    draft.body = Body::None;

    let example = overlay_of(&base(), &draft, "e9", "Page 2").unwrap();

    assert_eq!(
        example.query,
        vec![param("page", "2", true), param("extra", "1", true)]
    );
    assert_eq!(
        example.headers,
        vec![header("X-Tenant", "b"), header("X-Debug", "1")]
    );
    assert_eq!(example.body, Some(Body::None));
    assert_eq!(apply(&base(), &example), draft);
}

#[test]
fn an_overlay_cannot_hold_a_different_path_or_method() {
    let mut draft = base();
    draft.url = "{{base}}/accounts?page=1".into();
    assert_eq!(
        overlay_of(&base(), &draft, "e", "E")
            .unwrap_err()
            .to_string(),
        "An example keeps the request's method and path; change those in the request, or use a variable"
    );
    let mut method = base();
    method.method = "GET".into();
    assert!(overlay_of(&base(), &method, "e", "E").is_err());
}

#[test]
fn removed_query_parameters_and_headers_are_recorded_as_disabled() {
    let mut draft = base();
    draft.url = "{{base}}/users?page=1#top".into();
    draft.headers.remove(1);

    let example = overlay_of(&base(), &draft, "e", "E").unwrap();

    assert_eq!(example.query, vec![param("notify", "false", false)]);
    assert_eq!(
        example.headers,
        vec![Header {
            enabled: false,
            ..header("X-Tenant", "a")
        }]
    );
    assert_eq!(apply(&base(), &example).url, draft.url);
}
