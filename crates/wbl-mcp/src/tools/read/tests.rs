#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::json;
use wbl_core::auth::Auth;
use wbl_core::{Body, Header, Request};
use wbl_format::collection::{self, auth, environments, headers};
use wbl_format::{Environment, SecretName, StoredRequest};

use crate::test_support::harness;

fn stored(name: &str) -> StoredRequest {
    StoredRequest {
        name: name.into(),
        order: 0,
        request: Request {
            method: "GET".into(),
            url: "{{base}}/users".into(),
            headers: vec![Header {
                name: "Accept".into(),
                value: "application/json".into(),
                enabled: true,
            }],
            body: Body::None,
        },
        examples: vec![],
        settings: Default::default(),
        auth: Default::default(),
        variables: Default::default(),
        captures: vec![],
        assertions: vec![],
    }
}

#[tokio::test]
async fn collections_are_listed_by_name_and_folder() {
    let h = harness(&["API", "Admin"]).await;

    let listed = h.call("list_collections", json!({})).await;

    assert_eq!(listed["collections"][0]["name"], "API");
    assert_eq!(
        listed["collections"][1]["folder"],
        h.root(1).to_string_lossy().as_ref()
    );
}

#[tokio::test]
async fn the_tree_and_a_request_show_what_is_inherited() {
    let h = harness(&["API"]).await;
    let root = h.root(0);
    let folder = collection::create_folder(&root, "", "Users").unwrap();
    let path = collection::create_request(&root, &folder, &stored("List")).unwrap();
    headers::save_headers(
        &root,
        "",
        &[Header {
            name: "X-Tenant".into(),
            value: "acme".into(),
            enabled: true,
        }],
    )
    .unwrap();
    auth::save_auth(
        &root,
        &folder,
        &Auth::Bearer {
            token: "{{token}}".into(),
        },
    )
    .unwrap();

    let tree = h.call("get_tree", json!({ "collection": "API" })).await;
    let request = h
        .call("get_request", json!({ "collection": "API", "path": path }))
        .await;

    assert_eq!(tree["name"], "API");
    assert_eq!(tree["children"][0]["kind"], "folder");
    assert_eq!(tree["children"][0]["children"][0]["path"], path.as_str());
    assert_eq!(tree["headers"][0]["name"], "X-Tenant");
    assert_eq!(request["request"]["name"], "List");
    let sent: Vec<&str> = request["sentHeaders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["name"].as_str().unwrap())
        .collect();
    assert_eq!(sent, ["X-Tenant", "Accept"]);
    assert_eq!(request["effectiveAuth"]["type"], "bearer");
}

#[tokio::test]
async fn an_environment_shows_secret_names_never_values() {
    let h = harness(&["API"]).await;
    let root = h.root(0);
    let id = environments::create(
        &root,
        &Environment {
            name: "Dev".into(),
            protected: true,
            variables: [("base".to_owned(), "http://localhost".to_owned())].into(),
            secrets: vec![
                SecretName {
                    name: "token".into(),
                    description: "API token".into(),
                },
                SecretName {
                    name: "unset".into(),
                    description: String::new(),
                },
            ],
        },
    )
    .unwrap();
    let collection_id = collection::id(&root).unwrap();
    h.secrets
        .set(&collection_id, &id, "token", "sentinel-9d2e")
        .unwrap();

    let listed = h
        .call("list_environments", json!({ "collection": "API" }))
        .await;
    let env = h
        .call("get_environment", json!({ "collection": "API", "id": id }))
        .await;

    assert_eq!(listed["environments"][0]["name"], "Dev");
    assert_eq!(env["protected"], true);
    assert_eq!(env["variables"]["base"], "http://localhost");
    assert_eq!(
        env["secrets"][0],
        json!({"name": "token", "description": "API token", "set": true})
    );
    assert_eq!(env["secrets"][1]["set"], false);
    assert!(!env.to_string().contains("sentinel-9d2e"));
}

#[tokio::test]
async fn paths_outside_the_collection_and_unknown_collections_are_refused() {
    let h = harness(&["API"]).await;

    let escape = h
        .fails(
            "get_request",
            json!({ "collection": "API", "path": "../../etc/passwd" }),
        )
        .await;
    let unknown = h.fails("get_tree", json!({ "collection": "/tmp" })).await;
    let missing = h
        .fails(
            "get_request",
            json!({ "collection": "API", "path": "nope.req.toml" }),
        )
        .await;

    assert_eq!(escape, "invalid_path");
    assert_eq!(unknown, "not_reachable");
    assert_eq!(missing, "io");
}
