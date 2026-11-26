#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::interpolate::{Dynamic, Scope, Value};
use crate::{Body, Header, Request};

struct NoDynamic;
impl Dynamic for NoDynamic {
    fn value(&self, _: &str) -> Option<String> {
        None
    }
}

fn request() -> Request {
    Request {
        method: "GET".into(),
        url: "https://api.example.com/users?page=1".into(),
        headers: vec![],
        body: Body::None,
    }
}

fn scopes() -> Vec<Scope> {
    vec![Scope::new(
        [
            ("user".to_owned(), Value::Plain("ada".into())),
            ("pass".to_owned(), Value::Secret(Some("s3cret".into()))),
            ("token".to_owned(), Value::Secret(Some("tok-123".into()))),
        ]
        .into(),
    )]
}

fn resolved(auth: &Auth) -> ResolvedAuth {
    resolve_auth(auth, &scopes(), &NoDynamic).unwrap()
}

#[test]
fn the_nearest_auth_that_is_not_inherit_wins() {
    let bearer = Auth::Bearer { token: "t".into() };
    let basic = Auth::Basic {
        username: "u".into(),
        password: "p".into(),
    };

    assert_eq!(effective(&[&Auth::Inherit, &bearer, &basic]), &bearer);
    assert_eq!(effective(&[&basic, &bearer]), &basic);
    assert_eq!(effective(&[&Auth::Inherit, &Auth::Inherit]), &Auth::None);
    assert_eq!(effective(&[]), &Auth::None);
}

#[test]
fn basic_auth_is_encoded_and_both_forms_are_secret() {
    let auth = resolved(&Auth::Basic {
        username: "{{user}}".into(),
        password: "{{pass}}".into(),
    });

    let applied = apply(&request(), &auth, None).unwrap();

    assert_eq!(
        applied.headers,
        vec![Header {
            name: "Authorization".into(),
            value: "Basic YWRhOnMzY3JldA==".into(),
            enabled: true
        }]
    );
    assert!(auth.secrets.contains(&"s3cret".to_owned()));
    assert!(
        auth.secrets.contains(&"YWRhOnMzY3JldA==".to_owned()),
        "the encoded form is masked too"
    );
}

#[test]
fn bearer_and_api_keys_in_a_header_or_the_query() {
    let bearer = apply(
        &request(),
        &resolved(&Auth::Bearer {
            token: "{{token}}".into(),
        }),
        None,
    )
    .unwrap();
    let header = apply(
        &request(),
        &resolved(&Auth::ApiKey {
            name: "X-Api-Key".into(),
            value: "{{token}}".into(),
            location: KeyLocation::Header,
        }),
        None,
    )
    .unwrap();
    let query = apply(
        &request(),
        &resolved(&Auth::ApiKey {
            name: "api key".into(),
            value: "a&b=c".into(),
            location: KeyLocation::Query,
        }),
        None,
    )
    .unwrap();

    assert_eq!(bearer.headers[0].value, "Bearer tok-123");
    assert_eq!(
        header.headers[0],
        Header {
            name: "X-Api-Key".into(),
            value: "tok-123".into(),
            enabled: true
        }
    );
    assert_eq!(
        query.url,
        "https://api.example.com/users?page=1&api+key=a%26b%3Dc"
    );
}

#[test]
fn a_header_the_user_set_wins_over_the_auth() {
    let mut own = request();
    own.headers.push(Header {
        name: "authorization".into(),
        value: "Custom x".into(),
        enabled: true,
    });

    let applied = apply(&own, &resolved(&Auth::Bearer { token: "t".into() }), None).unwrap();

    assert_eq!(applied.headers.len(), 1);
    assert_eq!(applied.headers[0].value, "Custom x");
}

#[test]
fn oauth_needs_a_token_and_uses_it_as_bearer() {
    let config = OAuth2Config {
        grant: Grant::ClientCredentials,
        token_url: "https://idp/token".into(),
        auth_url: String::new(),
        device_url: String::new(),
        client_id: "app".into(),
        client_secret: "{{pass}}".into(),
        scope: "read".into(),
        redirect_uri: "http://localhost:8080/{{user}}".into(),
    };
    let auth = resolved(&Auth::OAuth2(config));

    assert_eq!(
        apply(&request(), &auth, None).unwrap_err(),
        AuthError::TokenRequired
    );
    let applied = apply(&request(), &auth, Some("access-1")).unwrap();
    assert_eq!(applied.headers[0].value, "Bearer access-1");
    let ResolvedAuthKind::OAuth2(resolved) = &auth.kind else {
        panic!()
    };
    assert_eq!(resolved.client_secret, "s3cret");
    assert_eq!(resolved.redirect_uri, "http://localhost:8080/ada");
}

#[test]
fn unknown_variables_in_auth_are_reported() {
    let error = resolve_auth(
        &Auth::Bearer {
            token: "{{missing}}".into(),
        },
        &scopes(),
        &NoDynamic,
    )
    .unwrap_err();
    assert_eq!(error.undefined, vec!["missing".to_owned()]);
}

#[test]
fn none_and_inherit_change_nothing() {
    assert_eq!(
        apply(&request(), &resolved(&Auth::None), None).unwrap(),
        request()
    );
    assert_eq!(
        apply(&request(), &resolved(&Auth::Inherit), None).unwrap(),
        request()
    );
}

#[test]
fn an_oauth_configuration_has_a_stable_key_for_its_cached_token() {
    let a = OAuth2Config {
        grant: Grant::AuthorizationCode,
        token_url: "https://idp/token".into(),
        auth_url: "https://idp/auth".into(),
        device_url: String::new(),
        client_id: "app".into(),
        client_secret: String::new(),
        scope: "read".into(),
        redirect_uri: String::new(),
    };
    let b = OAuth2Config {
        scope: "write".into(),
        ..a.clone()
    };

    assert_eq!(a.cache_key(), a.clone().cache_key());
    assert_ne!(a.cache_key(), b.cache_key());
    assert_eq!(a.cache_key().len(), 16);
}
