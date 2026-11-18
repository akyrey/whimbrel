#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::{Body, BodyKind, Header, Request};

struct Fixed;

impl Dynamic for Fixed {
    fn value(&self, name: &str) -> Option<String> {
        match name {
            "uuid" => Some("00000000-0000-4000-8000-000000000000".into()),
            "timestamp" => Some("1700000000".into()),
            _ => None,
        }
    }
}

fn scope(pairs: &[(&str, Value)]) -> Scope {
    Scope::new(
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    )
}

fn plain(text: &str) -> Value {
    Value::Plain(text.into())
}

fn request(url: &str) -> Request {
    Request {
        method: "POST".into(),
        url: url.into(),
        headers: vec![Header {
            name: "Authorization".into(),
            value: "Bearer {{token}}".into(),
            enabled: true,
        }],
        body: Body::Text {
            kind: BodyKind::Json,
            text: r#"{"id":"{{$uuid}}","at":{{ $timestamp }}}"#.into(),
        },
    }
}

#[test]
fn resolves_everywhere_and_lists_the_secrets_it_used() {
    let scopes = [
        scope(&[("base", plain("https://api.example.com"))]),
        scope(&[
            ("token", Value::Secret(Some("s3cret".into()))),
            ("base", plain("http://ignored")),
        ]),
    ];

    let resolved = resolve(&request("{{ base }}/users"), &scopes, &Fixed).unwrap();

    assert_eq!(resolved.request.url, "https://api.example.com/users");
    assert_eq!(resolved.request.headers[0].value, "Bearer s3cret");
    assert_eq!(
        resolved.request.body,
        Body::Text {
            kind: BodyKind::Json,
            text: r#"{"id":"00000000-0000-4000-8000-000000000000","at":1700000000}"#.into()
        }
    );
    assert_eq!(resolved.secrets, vec!["s3cret".to_owned()]);
}

#[test]
fn the_first_scope_wins() {
    let scopes = [
        scope(&[("v", plain("example"))]),
        scope(&[("v", plain("environment")), ("token", plain("t"))]),
    ];
    assert_eq!(
        resolve(&request("http://x/{{v}}"), &scopes, &Fixed)
            .unwrap()
            .request
            .url,
        "http://x/example"
    );
}

#[test]
fn unknown_names_and_unset_secrets_are_reported_together() {
    let scopes = [scope(&[("token", Value::Secret(None))])];

    let error = resolve(&request("{{base}}/{{other}}/{{base}}"), &scopes, &Fixed).unwrap_err();

    assert_eq!(
        error,
        ResolveError {
            undefined: vec!["base".into(), "other".into()],
            unset_secrets: vec!["token".into()]
        }
    );
    assert_eq!(
        error.to_string(),
        "Not defined: base, other. No value in the keychain for: token"
    );
}

#[test]
fn unknown_dynamic_values_are_undefined_and_disabled_headers_are_left_alone() {
    let mut req = request("http://x/{{$nope}}");
    req.headers[0].enabled = false;

    let error = resolve(&req, &[], &Fixed).unwrap_err();

    assert_eq!(error.undefined, vec!["$nope".to_owned()]);
    assert!(
        error.unset_secrets.is_empty(),
        "the disabled header's {{token}} is not needed"
    );
}

#[test]
fn text_that_is_not_a_variable_is_kept() {
    let scopes = [scope(&[("a", plain("1"))])];
    for (text, expected) in [
        ("{{a}}{{a}}", "11"),
        ("{ {a} }", "{ {a} }"),
        ("{{ not a name }}", "{{ not a name }}"),
        ("{{a", "{{a"),
        ("}}{{a}}", "}}1"),
        ("{{{a}}}", "{1}"),
        ("é{{a}}漢", "é1漢"),
    ] {
        assert_eq!(interpolate(text, &scopes, &Fixed).text, expected, "{text}");
    }
}

#[test]
fn names_used_lists_each_name_once_in_order() {
    let req = request("{{base}}/{{id}}/{{base}}");
    assert_eq!(
        names_used(&req),
        vec!["base", "id", "token", "$uuid", "$timestamp"]
    );
}

#[test]
fn masking_hides_every_secret_value() {
    let masked = mask(
        "Bearer s3cret and s3cret again, key=abc",
        &["s3cret".into(), "abc".into(), String::new()],
    );
    assert_eq!(masked, "Bearer •••••• and •••••• again, key=••••••");
}

#[test]
fn system_dynamic_values_look_right() {
    let system = SystemDynamic;
    assert_eq!(system.value("uuid").unwrap().len(), 36);
    assert!(system.value("timestamp").unwrap().parse::<u64>().unwrap() > 1_700_000_000);
    assert!(system.value("isoTimestamp").unwrap().ends_with('Z'));
    assert!(system.value("randomInt").unwrap().parse::<u32>().unwrap() <= 1000);
    assert_eq!(system.value("other"), None);
}
