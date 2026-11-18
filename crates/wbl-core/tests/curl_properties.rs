#![allow(clippy::unwrap_used)]
//! Whatever the request, the exported command reads back as the same request: the quoting is
//! safe for any text (quotes, backslashes, `$`, backticks, line breaks).

use proptest::prelude::*;
use wbl_core::curl::{parse, to_curl};
use wbl_core::{Body, BodyKind, Header, Request};

fn text() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 '\"\\\\$`!*\\n\\t{}:;&|<>é]{0,30}").unwrap()
}

fn request() -> impl Strategy<Value = Request> {
    let header =
        ("X-[A-Za-z0-9-]{1,10}", "[^\\s\\r\\n][^\\r\\n]{0,20}").prop_map(|(name, value)| Header {
            name,
            value,
            enabled: true,
        });
    let body = prop_oneof![
        Just(Body::None),
        text().prop_map(|t| Body::Text {
            kind: BodyKind::Text,
            text: format!("t{t}")
        }),
        text().prop_map(|t| Body::Text {
            kind: BodyKind::Json,
            text: format!("{{{t}")
        }),
    ];
    (
        "[A-Z]{1,7}",
        "https?://[a-z0-9.]{1,10}/[a-zA-Z0-9 '\"$&?=/]{0,20}",
        prop::collection::vec(header, 0..4),
        body,
    )
        .prop_map(|(method, url, headers, body)| Request {
            method,
            url,
            headers,
            body,
        })
}

proptest! {
    #[test]
    fn exported_commands_read_back_the_same(request in request()) {
        let command = to_curl(&request);
        let back = parse(&command).unwrap();
        prop_assert!(back.ignored.is_empty(), "{:?}", back.ignored);
        prop_assert_eq!(back.request, request);
    }
}
