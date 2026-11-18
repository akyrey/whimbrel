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

#[test]
fn exports_a_command_that_quotes_everything_safely() {
    let request = Request {
        method: "POST".into(),
        url: "https://api.example.com/users?q=it's".into(),
        headers: vec![
            header("Content-Type", "application/json"),
            Header {
                enabled: false,
                ..header("X-Off", "1")
            },
        ],
        body: Body::Text {
            kind: BodyKind::Json,
            text: "{\"name\":\"O'Hara\"}".into(),
        },
    };

    let command = to_curl(&request);

    assert_eq!(
        command,
        "curl -X POST 'https://api.example.com/users?q=it'\\''s' \\\n  -H 'Content-Type: application/json' \\\n  --data-raw '{\"name\":\"O'\\''Hara\"}'"
    );
}

#[test]
fn a_get_without_body_is_one_line() {
    let request = Request {
        method: "GET".into(),
        url: "http://x".into(),
        headers: vec![],
        body: Body::None,
    };
    assert_eq!(to_curl(&request), "curl 'http://x'");
}

#[test]
fn imports_what_browsers_copy() {
    let text = r#"curl 'https://api.example.com/users' \
  -H 'accept: application/json' \
  -H "Authorization: Bearer abc" \
  --data-raw '{"name":"ada"}' \
  --compressed"#;

    let imported = parse(text).unwrap();

    assert_eq!(imported.request.method, "POST");
    assert_eq!(imported.request.url, "https://api.example.com/users");
    assert_eq!(
        imported.request.headers,
        vec![
            header("accept", "application/json"),
            header("Authorization", "Bearer abc")
        ]
    );
    assert_eq!(
        imported.request.body,
        Body::Text {
            kind: BodyKind::Json,
            text: r#"{"name":"ada"}"#.into()
        }
    );
    assert!(imported.ignored.is_empty(), "{:?}", imported.ignored);
}

#[test]
fn understands_the_common_flags() {
    let imported = parse("curl -X PUT -u ada:secret -A agent/1 -b 'a=1' -e http://ref -k -L --url http://x/y -d 'a=1' -d b=2").unwrap();

    assert_eq!(imported.request.method, "PUT");
    assert_eq!(imported.request.url, "http://x/y");
    assert_eq!(
        imported.request.headers,
        vec![
            header("Authorization", "Basic YWRhOnNlY3JldA=="),
            header("User-Agent", "agent/1"),
            header("Cookie", "a=1"),
            header("Referer", "http://ref"),
        ]
    );
    assert_eq!(
        imported.request.body,
        Body::Text {
            kind: BodyKind::Text,
            text: "a=1&b=2".into()
        }
    );
    assert!(!imported.settings.verify_tls);
    assert!(imported.settings.follow_redirects);
}

#[test]
fn json_head_and_unknown_flags() {
    let json = parse(r#"curl --json '{"a":1}' http://x"#).unwrap();
    let head = parse("curl -I http://x --max-time 5 -o out.json").unwrap();

    assert_eq!(json.request.method, "POST");
    assert_eq!(
        json.request.headers,
        vec![
            header("Content-Type", "application/json"),
            header("Accept", "application/json")
        ]
    );
    assert_eq!(head.request.method, "HEAD");
    assert_eq!(
        head.ignored,
        vec!["--max-time 5".to_owned(), "-o out.json".to_owned()]
    );
}

#[test]
fn refuses_what_is_not_a_curl_command() {
    assert_eq!(parse("wget http://x").unwrap_err(), CurlError::NotCurl);
    assert_eq!(parse("curl -H 'a: b'").unwrap_err(), CurlError::NoUrl);
    assert_eq!(
        parse("curl 'http://x").unwrap_err(),
        CurlError::Unterminated
    );
}

#[test]
fn what_is_exported_imports_back() {
    let request = Request {
        method: "PATCH".into(),
        url: "http://x/a b".into(),
        headers: vec![header("X-Quote", "it's \"fine\"")],
        body: Body::Text {
            kind: BodyKind::Text,
            text: "line 1\nline 2 $HOME `x` \\".into(),
        },
    };

    let back = parse(&to_curl(&request)).unwrap();

    assert_eq!(back.request, request);
}
