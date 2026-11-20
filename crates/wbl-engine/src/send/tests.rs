#![allow(clippy::unwrap_used, clippy::expect_used)]

use wbl_core::auth::Auth;
use wbl_format::collection::{self, auth, environments, headers};
use wbl_format::{Environment, SecretName};

use super::*;
use crate::resolve::Source;
use crate::test_support::{echo_server, fixture};

fn header(name: &str, value: &str) -> Header {
    Header {
        name: name.into(),
        value: value.into(),
        enabled: true,
    }
}

/// An environment `Dev` with `host` plain and `key` secret (its value in the keychain).
fn dev(f: &crate::test_support::Fixture, key: &str) -> String {
    let root = f.root();
    let id = environments::create(
        &root,
        &Environment {
            name: "Dev".into(),
            protected: false,
            variables: [("host".to_owned(), "example.test".to_owned())].into(),
            secrets: vec![SecretName {
                name: "key".into(),
                description: String::new(),
            }],
        },
    )
    .unwrap();
    f.secrets.set(&f.collection_id(), &id, "key", key).unwrap();
    id
}

#[tokio::test]
async fn sends_with_inherited_headers_variables_and_auth() {
    let f = fixture();
    let root = f.root();
    let base = echo_server().await;
    let env = dev(&f, "s3cret-value");
    let folder = collection::create_folder(&root, "", "Users").unwrap();
    headers::save_headers(&root, "", &[header("X-Tenant", "acme")]).unwrap();
    headers::save_headers(&root, &folder, &[header("X-Folder", "{{host}}")]).unwrap();
    auth::save_auth(
        &root,
        "",
        &Auth::Bearer {
            token: "{{key}}".into(),
        },
    )
    .unwrap();
    let path = format!("{folder}/list.toml");
    let args = RequestSendArgs {
        environment: Some(env),
        path: Some(path),
        ..f.args(&format!("{base}/users"))
    };

    let sent = f.send(args).await.unwrap();

    let echoed = sent.response.body.text.unwrap().to_ascii_lowercase();
    assert!(echoed.contains("x-tenant: acme"));
    assert!(echoed.contains("x-folder: example.test"));
    assert!(echoed.contains("authorization: bearer s3cret-value"));
}

#[test]
fn preview_and_curl_mask_secrets_unless_revealed() {
    let f = fixture();
    let root = f.root();
    let env = dev(&f, "s3cret-value");
    let args = RequestSendArgs {
        environment: Some(env),
        request: Request {
            headers: vec![header("X-Key", "{{key}}")],
            ..f.args("http://{{host}}/a").request
        },
        ..f.args("")
    };
    let sources = f.sources(&root);

    let shown = preview(&sources, &args).unwrap();
    let masked_curl = curl(&sources, &args, false).unwrap();
    let revealed = curl(&sources, &args, true).unwrap();

    assert_eq!(shown.url, "http://example.test/a");
    assert_ne!(shown.headers[0].value, "s3cret-value");
    assert!(!masked_curl.contains("s3cret-value"));
    assert!(revealed.contains("s3cret-value"));
}

#[test]
fn variables_list_secrets_without_their_values() {
    let f = fixture();
    let root = f.root();
    let env = dev(&f, "s3cret-value");
    let args = RequestSendArgs {
        environment: Some(env),
        ..f.args("http://x")
    };

    let found = variables(&f.sources(&root), &args).unwrap();

    let key = found.iter().find(|v| v.name == "key").unwrap();
    assert!(key.secret && key.set && key.value.is_none());
    assert_eq!(key.source, Source::Environment);
    let host = found.iter().find(|v| v.name == "host").unwrap();
    assert_eq!(host.value.as_deref(), Some("example.test"));
    assert!(found.iter().any(|v| v.source == Source::Dynamic));
}

#[test]
fn an_undefined_variable_is_named_before_any_send() {
    let f = fixture();
    let root = f.root();

    let error = prepare(&f.sources(&root), &f.args("http://{{nowhere}}/")).unwrap_err();

    assert_eq!(error.code, "unresolved");
    assert_eq!(
        error.detail.unwrap()["undefined"],
        serde_json::json!(["nowhere"])
    );
}

#[tokio::test]
async fn a_missing_tls_file_fails_the_send() {
    let f = fixture();
    let root = f.root();
    collection::auth::save_tls(
        &root,
        &wbl_format::CollectionTls {
            ca_file: "missing.pem".into(),
            ..Default::default()
        },
    )
    .unwrap();

    let error = ready(&f.oauth, &f.sources(&root), &f.args("http://127.0.0.1:1/"))
        .await
        .err()
        .unwrap();

    assert_eq!(error.code, "tls");
}

#[tokio::test]
async fn a_request_outside_a_collection_has_no_jar() {
    let f = fixture();
    let sources = Sources {
        root: None,
        runtime: &f.runtime,
        secrets: &f.secrets,
    };
    let args = RequestSendArgs {
        root: None,
        ..f.args("http://127.0.0.1:1/")
    };

    let ready = ready(&f.oauth, &sources, &args).await.unwrap();

    assert_eq!(ready.exchange.jar, None);
    assert_eq!(ready.resolved.request.url, "http://127.0.0.1:1/");
}
