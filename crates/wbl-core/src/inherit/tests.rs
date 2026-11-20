#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::{Body, Header, Request};

fn header(name: &str, value: &str) -> Header {
    Header {
        name: name.into(),
        value: value.into(),
        enabled: true,
    }
}

fn off(name: &str, value: &str) -> Header {
    Header {
        enabled: false,
        ..header(name, value)
    }
}

fn request(headers: Vec<Header>) -> Request {
    Request {
        method: "GET".into(),
        url: "https://example.com".into(),
        headers,
        body: Body::None,
    }
}

#[test]
fn a_request_without_inherited_headers_is_unchanged() {
    let own = request(vec![header("Accept", "text/plain")]);

    assert_eq!(with_inherited(&own, &[]), own);
    assert_eq!(with_inherited(&own, &[vec![], vec![]]), own);
}

#[test]
fn inherited_headers_come_first_from_the_collection_inwards() {
    // Innermost folder first, the collection last, like the auth chain.
    let chain = vec![
        vec![header("X-Folder", "inner")],
        vec![
            header("X-Tenant", "{{tenant}}"),
            header("Accept", "application/json"),
        ],
    ];
    let own = request(vec![header("X-Request", "1")]);

    let sent = with_inherited(&own, &chain);

    assert_eq!(
        sent.headers,
        vec![
            header("X-Tenant", "{{tenant}}"),
            header("Accept", "application/json"),
            header("X-Folder", "inner"),
            header("X-Request", "1"),
        ]
    );
}

#[test]
fn a_deeper_header_replaces_every_inherited_one_of_its_name_ignoring_case() {
    let chain = vec![
        vec![header("accept", "text/csv")],
        vec![
            header("Accept", "application/json"),
            header("ACCEPT", "*/*"),
        ],
    ];

    let sent = with_inherited(&request(vec![]), &chain);

    assert_eq!(sent.headers, vec![header("accept", "text/csv")]);
}

#[test]
fn the_request_overrides_inherited_headers_and_keeps_its_duplicates() {
    let chain = vec![vec![header("Cookie", "a=1"), header("X-Keep", "yes")]];
    let own = request(vec![header("Cookie", "b=2"), header("cookie", "c=3")]);

    let sent = with_inherited(&own, &chain);

    assert_eq!(
        sent.headers,
        vec![
            header("X-Keep", "yes"),
            header("Cookie", "b=2"),
            header("cookie", "c=3"),
        ]
    );
}

#[test]
fn a_disabled_header_turns_an_inherited_one_off() {
    let chain = vec![vec![header("X-Debug", "1")]];
    let own = request(vec![off("X-Debug", "")]);

    let sent = with_inherited(&own, &chain);

    assert_eq!(sent.headers, vec![off("X-Debug", "")]);
}

#[test]
fn inherited_lists_what_a_request_receives_before_its_own_headers() {
    let chain = vec![
        vec![off("X-Trace", "")],
        vec![header("X-Trace", "1"), header("X-Tenant", "t")],
    ];

    assert_eq!(
        inherited(&chain),
        vec![header("X-Tenant", "t"), off("X-Trace", "")]
    );
}
