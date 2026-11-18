#![allow(clippy::unwrap_used, clippy::expect_used)]

use rstest::rstest;

use super::*;

fn get(url: &str) -> Request {
    Request {
        method: "GET".into(),
        url: url.into(),
        headers: vec![],
        body: Body::None,
    }
}

fn header(name: &str, value: &str) -> Header {
    Header {
        name: name.into(),
        value: value.into(),
        enabled: true,
    }
}

#[rstest]
#[case("GET")]
#[case("QUERY")]
#[case("PROPFIND")]
#[case("x-custom.1")]
fn any_token_is_a_method(#[case] method: &str) {
    let request = Request {
        method: method.into(),
        ..get("http://localhost/")
    };
    assert_eq!(request.validate(), Ok(()));
}

#[rstest]
#[case("", "The method is empty")]
#[case("GE T", "The method \"GE T\" is not a valid HTTP method")]
#[case("GET\n", "The method \"GET\\n\" is not a valid HTTP method")]
fn a_bad_method_is_refused(#[case] method: &str, #[case] message: &str) {
    let request = Request {
        method: method.into(),
        ..get("http://localhost/")
    };
    assert_eq!(
        request.validate(),
        Err(RequestError::Method(message.into()))
    );
}

#[rstest]
#[case("", "Enter a URL")]
#[case("localhost:8080/users", "The URL needs http:// or https://")]
#[case("ftp://example.com", "The URL needs http:// or https://")]
#[case("http://", "The URL is not valid: empty host")]
fn a_bad_url_is_refused(#[case] url: &str, #[case] message: &str) {
    assert_eq!(get(url).validate(), Err(RequestError::Url(message.into())));
}

#[test]
fn headers_keep_their_order_and_duplicates_and_skip_disabled_ones() {
    let request = Request {
        headers: vec![
            header("Accept", "a"),
            Header {
                enabled: false,
                ..header("X-Off", "b")
            },
            header("Accept", "c"),
        ],
        ..get("https://example.com")
    };

    let sent: Vec<_> = request.enabled_headers().collect();

    assert_eq!(sent, vec![&header("Accept", "a"), &header("Accept", "c")]);
}

#[rstest]
#[case(header("Bad Name", "x"), "The header name \"Bad Name\" is not valid")]
#[case(header("", "x"), "A header has no name")]
#[case(
    header("X-Ok", "line\r\nInjected: 1"),
    "The value of X-Ok has a line break"
)]
fn a_bad_header_is_refused(#[case] bad: Header, #[case] message: &str) {
    let request = Request {
        headers: vec![bad],
        ..get("https://example.com")
    };
    assert_eq!(
        request.validate(),
        Err(RequestError::Header(message.into()))
    );
}

#[test]
fn a_disabled_bad_header_does_not_block_sending() {
    let request = Request {
        headers: vec![Header {
            enabled: false,
            ..header("Bad Name", "x")
        }],
        ..get("https://example.com")
    };
    assert_eq!(request.validate(), Ok(()));
}

#[test]
fn a_body_has_a_default_content_type_unless_one_is_set() {
    let json = Request {
        method: "POST".into(),
        body: Body::Text {
            kind: BodyKind::Json,
            text: "{}".into(),
        },
        ..get("https://example.com")
    };
    let explicit = Request {
        headers: vec![header("content-type", "application/vnd.api+json")],
        ..json.clone()
    };

    assert_eq!(json.default_content_type(), Some("application/json"));
    assert_eq!(explicit.default_content_type(), None);
    assert_eq!(get("https://example.com").default_content_type(), None);
}

#[test]
fn serialises_for_the_frontend_with_tagged_bodies() {
    let request = Request {
        body: Body::Text {
            kind: BodyKind::Xml,
            text: "<a/>".into(),
        },
        ..get("https://example.com")
    };

    let json = serde_json::to_value(&request).unwrap();

    assert_eq!(json["body"]["type"], "text");
    assert_eq!(json["body"]["kind"], "xml");
    assert_eq!(serde_json::from_value::<Request>(json).unwrap(), request);
}
