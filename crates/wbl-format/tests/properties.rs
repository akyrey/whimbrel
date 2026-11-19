#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 2.2: whatever the request, a file round-trips, an unchanged save is byte-identical, and
//! an edit touches only its own line.

use proptest::prelude::*;
use wbl_core::example::{Example, Param};
use wbl_core::{Body, BodyKind, Header, Request};
use wbl_format::{RequestFile, StoredRequest};

/// Any text, including quotes, backslashes, line breaks, tabs and non-ASCII.
fn text() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 \"'\\\\\\n\\r\\t{}:,é漢🙂]{0,40}").unwrap()
}

fn header() -> impl Strategy<Value = Header> {
    ("[A-Za-z][A-Za-z0-9-]{0,15}", text(), any::<bool>()).prop_map(|(name, value, enabled)| {
        Header {
            name,
            value,
            enabled,
        }
    })
}

fn body() -> impl Strategy<Value = Body> {
    prop_oneof![
        Just(Body::None),
        (
            prop_oneof![
                Just(BodyKind::Json),
                Just(BodyKind::Xml),
                Just(BodyKind::Text)
            ],
            text()
        )
            .prop_map(|(kind, text)| Body::Text { kind, text }),
    ]
}

/// URLs are one line (a line break would make the request invalid).
fn url() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9:/.{}?=&%_ -]{0,40}"
}

fn example() -> impl Strategy<Value = Example> {
    (
        "[a-z0-9]{1,8}",
        text(),
        prop::collection::btree_map("[a-zA-Z][a-zA-Z0-9_]{0,8}", text(), 0..3),
        prop::collection::vec(("[a-z]{1,6}", text(), any::<bool>()), 0..3),
        prop::collection::vec(header(), 0..3),
        prop::option::of(body()),
    )
        .prop_map(|(id, name, variables, query, headers, body)| Example {
            variables,
            query: query
                .into_iter()
                .map(|(name, value, enabled)| Param {
                    name,
                    value,
                    enabled,
                })
                .collect(),
            headers,
            body,
            ..Example::named(&id, &name)
        })
}

fn auth() -> impl Strategy<Value = wbl_core::auth::Auth> {
    use wbl_core::auth::{Auth, Grant, KeyLocation, OAuth2Config};
    let word = "[a-zA-Z0-9{}_-]{1,12}";
    prop_oneof![
        Just(Auth::Inherit),
        Just(Auth::None),
        (word, text()).prop_map(|(username, password)| Auth::Basic { username, password }),
        word.prop_map(|token| Auth::Bearer { token }),
        (word, text(), any::<bool>()).prop_map(|(name, value, query)| Auth::ApiKey {
            name,
            value,
            location: if query {
                KeyLocation::Query
            } else {
                KeyLocation::Header
            },
        }),
        (url(), word, text(), text()).prop_map(|(token_url, client_id, client_secret, scope)| {
            Auth::OAuth2(OAuth2Config {
                grant: Grant::ClientCredentials,
                token_url,
                auth_url: String::new(),
                device_url: String::new(),
                client_id,
                client_secret,
                scope,
            })
        }),
    ]
}

fn stored() -> impl Strategy<Value = StoredRequest> {
    (
        (text(), any::<i64>(), "[A-Z]{1,8}", url()),
        prop::collection::vec(header(), 0..4),
        body(),
        prop::collection::vec(example(), 0..3),
        (
            prop::option::of(1u64..100_000),
            any::<bool>(),
            any::<bool>(),
        ),
        auth(),
        prop::collection::btree_map("[a-zA-Z][a-zA-Z0-9_]{0,8}", text(), 0..3),
    )
        .prop_map(
            |((name, order, method, url), headers, body, examples, settings, auth, variables)| {
                let (timeout_ms, follow_redirects, verify_tls) = settings;
                StoredRequest {
                    name,
                    order,
                    request: Request {
                        method,
                        url,
                        headers,
                        body,
                    },
                    examples,
                    settings: wbl_core::RequestSettings {
                        timeout_ms,
                        follow_redirects,
                        verify_tls,
                    },
                    auth,
                    variables,
                }
            },
        )
}

proptest! {
    #[test]
    fn written_files_read_back_the_same(request in stored()) {
        let text = RequestFile::new(&request).to_string();
        prop_assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), request);
    }

    #[test]
    fn an_unchanged_save_is_byte_identical(request in stored()) {
        let text = RequestFile::new(&request).to_string();
        let file = RequestFile::parse(&text).unwrap();
        prop_assert_eq!(file.updated(&request).to_string(), text);
    }

    #[test]
    fn changing_the_url_changes_one_line(request in stored(), url in "[a-z:/.{}]{1,30}") {
        prop_assume!(url != request.request.url);
        let before = RequestFile::new(&request).to_string();
        let edited = StoredRequest {
            request: Request { url: url.clone(), ..request.request.clone() },
            ..request.clone()
        };

        let after = RequestFile::parse(&before).unwrap().updated(&edited).to_string();

        let changed: Vec<_> = before
            .lines()
            .zip(after.lines())
            .filter(|(a, b)| a != b)
            .collect();
        prop_assert_eq!(before.lines().count(), after.lines().count());
        prop_assert_eq!(changed.len(), 1);
        prop_assert!(changed[0].1.starts_with("url = "));
    }

    #[test]
    fn any_edit_reads_back(request in stored(), other in stored()) {
        let file = RequestFile::new(&request);
        let text = file.updated(&other).to_string();
        prop_assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), other);
    }
}
