#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Task 8.4: Postman v2.1 and `.http` in, `.http` out.

use std::path::PathBuf;

use wbl_core::auth::{Auth, KeyLocation};
use wbl_core::{Body, BodyKind};
use wbl_interop::{Imported, ImportedRequest, InteropError, http_export, install, read};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn find<'a>(imported: &'a Imported, name: &str) -> &'a ImportedRequest {
    imported
        .requests
        .iter()
        .find(|r| r.stored.name == name)
        .unwrap()
}

#[test]
fn a_postman_collection_imports_with_folders_bodies_auth_and_examples() {
    let shop = read("shop.json", &fixture("postman.json")).unwrap();

    assert_eq!(shop.name, "Shop");
    assert_eq!(
        shop.folders,
        [
            vec!["Orders".to_owned()],
            vec!["Orders".into(), "Nested".into()]
        ]
    );
    assert_eq!(
        shop.auth,
        Auth::Bearer {
            token: "{{bearerToken}}".into()
        },
        "the pasted token is not copied"
    );
    assert_eq!(shop.variables["baseUrl"], "https://shop.test");
    assert!(!shop.variables.contains_key("apiSecret"));
    let secrets: Vec<_> = shop.environments[0]
        .secrets
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(secrets, ["apiSecret", "bearerToken"]);

    let list = find(&shop, "List orders");
    assert_eq!(list.folder, ["Orders"]);
    assert_eq!(
        list.stored.request.url,
        "{{baseUrl}}/orders?page=1&id={{$uuid}}"
    );
    assert!(!list.stored.request.headers[1].enabled);
    assert_eq!(list.stored.examples.len(), 1);
    assert_eq!(list.stored.examples[0].name, "Second page");
    assert_eq!(list.stored.examples[0].query[0].value, "2");

    let create = find(&shop, "Create order");
    assert_eq!(create.folder, ["Orders", "Nested"]);
    assert_eq!(create.stored.request.method, "POST");
    assert!(
        matches!(&create.stored.request.body, Body::Text { kind: BodyKind::Json, text } if text.contains("{{sku}}"))
    );
    assert_eq!(
        create.stored.auth,
        Auth::ApiKey {
            name: "X-Key".into(),
            value: "{{shopKey}}".into(),
            location: KeyLocation::Header
        }
    );

    let upload = find(&shop, "Upload");
    assert_eq!(upload.stored.auth, Auth::None);
    assert_eq!(upload.stored.request.url, "https://shop.test/upload?v=2");
    let Body::Multipart { fields } = &upload.stored.request.body else {
        panic!()
    };
    assert!(fields[1].file && fields[1].value == "/tmp/a.pdf");
    assert!(
        matches!(&find(&shop, "Login").stored.request.body, Body::Form { fields } if fields[1].value == "{{password}}")
    );
    assert!(
        matches!(&find(&shop, "Query").stored.request.body, Body::Text { text, .. } if text.contains("\"query\": \"{ me { id } }\""))
    );
    assert_eq!(
        find(&shop, "Bare").stored.request.url,
        "https://shop.test/health"
    );

    for expected in ["scripts", "hawk", "Other path", "Imported environment"] {
        assert!(
            shop.warnings.iter().any(|w| w.contains(expected)),
            "{expected}: {:?}",
            shop.warnings
        );
    }
}

#[test]
fn a_postman_environment_keeps_secret_names_only() {
    let env = read("staging.json", &fixture("environment.json")).unwrap();

    let staging = &env.environments[0];
    assert_eq!(staging.name, "Staging");
    assert_eq!(staging.variables.len(), 1);
    assert_eq!(staging.secrets[0].name, "shopKey");
    assert!(!format!("{env:?}").contains("real-secret"));
}

#[test]
fn other_json_is_refused() {
    assert_eq!(
        read("x.json", "{ nope").unwrap_err().code(),
        "interop_syntax"
    );
    assert_eq!(
        read("x.json", r#"{"info": {"schema": "v1"}}"#).unwrap_err(),
        InteropError::NotPostman
    );
    assert_eq!(
        read("x.json", r#"{"a": 1}"#).unwrap_err(),
        InteropError::NotPostman
    );
    let empty = r#"{"info": {"schema": "https://schema.getpostman.com/json/collection/v2.1.0/"}, "item": []}"#;
    assert_eq!(read("x.json", empty).unwrap_err(), InteropError::Empty);
    assert_eq!(
        read("x.http", "# only a comment\n").unwrap_err(),
        InteropError::Empty
    );
}

#[test]
fn an_http_file_imports() {
    let http = read("dir/requests.http", &fixture("requests.http")).unwrap();

    assert_eq!(http.name, "requests");
    assert_eq!(http.variables["host"], "https://api.test");
    let names: Vec<_> = http
        .requests
        .iter()
        .map(|r| r.stored.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["List users", "createUser", "GET {{host}}/health", "Upload"]
    );
    assert_eq!(
        http.requests[0].stored.request.url,
        "{{host}}/users?page=2&size=10"
    );
    let create = &http.requests[1].stored.request;
    assert_eq!(create.headers[1].value, "Bearer {{token}}");
    assert!(
        matches!(&create.body, Body::Text { kind: BodyKind::Json, text } if text.ends_with('}'))
    );
    assert_eq!(
        http.requests[3].stored.request.body,
        Body::File {
            path: "./data/file.bin".into()
        }
    );
    assert!(http.warnings[0].contains("Response handlers"));
}

#[test]
fn http_export_reads_back() {
    let http = read("requests.http", &fixture("requests.http")).unwrap();
    let shop = read("shop.json", &fixture("postman.json")).unwrap();
    let mut requests: Vec<_> = http
        .requests
        .iter()
        .map(|r| (r.stored.clone(), Auth::Inherit))
        .collect();
    requests.push((
        find(&shop, "Login").stored.clone(),
        Auth::Bearer {
            token: "{{t}}".into(),
        },
    ));
    requests.push((find(&shop, "Upload").stored.clone(), Auth::Inherit));
    requests.push((find(&shop, "Create order").stored.clone(), Auth::Inherit));

    let text = http_export::write(&requests);
    let back = read("out.http", &text).unwrap();

    assert_eq!(
        back.requests[0].stored.request.url,
        "{{host}}/users?page=2&size=10"
    );
    assert_eq!(
        back.requests[3].stored.request.body,
        Body::File {
            path: "./data/file.bin".into()
        }
    );
    let login = &back.requests[4].stored.request;
    assert!(
        login.headers.iter().any(|h| h.value == "Bearer {{t}}"),
        "inherited auth becomes a header: {text}"
    );
    assert!(matches!(&login.body, Body::Text { text, .. } if text == "user=ada&pass={{password}}"));
    assert!(
        text.contains(
            "Content-Disposition: form-data; name=\"doc\"; filename=\"a.pdf\"\n\n< /tmp/a.pdf"
        ),
        "{text}"
    );
    assert!(text.contains("X-Key: {{shopKey}}"), "{text}");
}

#[test]
fn installing_writes_folders_requests_variables_and_environments() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    wbl_format::collection::create(root, "Mine").unwrap();
    let folder = wbl_format::collection::create_folder(root, "", "Shop").unwrap();
    let shop = read("shop.json", &fixture("postman.json")).unwrap();

    let installed = install(root, &folder, &shop).unwrap();
    let again = install(root, &folder, &shop).unwrap();

    assert_eq!(installed.requests.len(), 6);
    assert!(
        installed
            .requests
            .iter()
            .any(|p| p == "shop/orders/nested/create-order.req.toml"),
        "{:?}",
        installed.requests
    );
    assert_eq!(installed.environments, ["imported"]);
    assert_eq!(
        wbl_format::collection::variables(root).unwrap()["baseUrl"],
        "https://shop.test"
    );
    assert_eq!(
        wbl_format::collection::auth::auth_of(root, &folder).unwrap(),
        shop.auth
    );
    assert!(again.environments.is_empty());
    assert!(again.warnings.iter().any(|w| w.contains("Imported exists")));
}

fn postman_oauth(name: &str, redirect_uri: &str) -> String {
    format!(
        r#"{{"name": "{name}", "request": {{"method": "GET", "url": "https://api.test/{name}", "auth": {{"type": "oauth2", "oauth2": [
            {{"key": "grant_type", "value": "authorization_code_with_pkce"}},
            {{"key": "accessTokenUrl", "value": "https://idp.test/token"}},
            {{"key": "authUrl", "value": "https://idp.test/authorize"}},
            {{"key": "redirect_uri", "value": "{redirect_uri}"}}
        ]}}}}}}"#
    )
}

fn redirect_of(imported: &Imported, name: &str) -> String {
    match &find(imported, name).stored.auth {
        Auth::OAuth2(c) => c.redirect_uri.clone(),
        other => panic!("not OAuth 2: {other:?}"),
    }
}

#[test]
fn a_postman_callback_url_is_kept_when_whimbrel_can_listen_on_it() {
    let items = [
        postman_oauth("loopback", "http://localhost:8080/cb"),
        postman_oauth("variable", "{{callback}}"),
        postman_oauth("hosted", "https://oauth.pstmn.io/v1/callback"),
    ]
    .join(",");
    let text = format!(
        r#"{{"info": {{"name": "C", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"}}, "item": [{items}]}}"#
    );

    let imported = read("c.json", &text).unwrap();

    assert_eq!(
        redirect_of(&imported, "loopback"),
        "http://localhost:8080/cb"
    );
    assert_eq!(redirect_of(&imported, "variable"), "{{callback}}");
    assert_eq!(
        redirect_of(&imported, "hosted"),
        "",
        "a random loopback port instead"
    );
    assert!(
        imported
            .warnings
            .iter()
            .any(|w| w.contains("https://oauth.pstmn.io/v1/callback")),
        "{:?}",
        imported.warnings
    );
}
