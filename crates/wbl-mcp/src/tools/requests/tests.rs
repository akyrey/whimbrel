#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::json;
use wbl_format::collection::{self, environments};
use wbl_format::{Environment, SecretName};

use crate::test_support::{Harness, harness};

async fn create(h: &Harness, folder: &str, name: &str) -> String {
    let created = h
        .call(
            "create_request",
            json!({
                "collection": "API",
                "folder": folder,
                "request": {
                    "name": name,
                    "request": {
                        "method": "POST",
                        "url": "{{base}}/users",
                        "headers": [{ "name": "Accept", "value": "application/json" }],
                        "body": { "type": "text", "kind": "json", "text": "{\"a\":1}" }
                    }
                }
            }),
        )
        .await;
    created["path"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn a_request_is_created_with_defaults_and_updated_field_by_field() {
    let h = harness(&["API"]).await;
    let path = create(&h, "", "Create user").await;

    let updated = h
        .call(
            "update_request",
            json!({
                "collection": "API",
                "path": path,
                "request": { "request": { "url": "{{base}}/people" }, "variables": { "id": "7" } }
            }),
        )
        .await;

    let request = &updated["request"];
    assert_eq!(request["name"], "Create user");
    assert_eq!(request["request"]["method"], "POST", "kept");
    assert_eq!(request["request"]["url"], "{{base}}/people");
    assert_eq!(request["request"]["headers"][0]["enabled"], true);
    assert_eq!(request["variables"]["id"], "7");
    assert_eq!(request["auth"]["type"], "inherit");
}

#[tokio::test]
async fn an_unchanged_update_leaves_the_file_byte_identical() {
    let h = harness(&["API"]).await;
    let root = h.root(0);
    let path = create(&h, "", "Create user").await;
    let before = std::fs::read(root.join(&path)).unwrap();
    let read = h
        .call("get_request", json!({ "collection": "API", "path": path }))
        .await;

    h.call(
        "update_request",
        json!({ "collection": "API", "path": path, "request": read["request"] }),
    )
    .await;

    assert_eq!(std::fs::read(root.join(&path)).unwrap(), before);
}

#[tokio::test]
async fn folders_and_requests_are_renamed_moved_ordered_copied_and_deleted() {
    let h = harness(&["API"]).await;
    let root = h.root(0);
    let users = h
        .call(
            "create_folder",
            json!({ "collection": "API", "name": "Users" }),
        )
        .await["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let admin = h
        .call(
            "create_folder",
            json!({ "collection": "API", "name": "Admin" }),
        )
        .await["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = create(&h, &users, "List").await;

    let renamed = h
        .call(
            "rename",
            json!({ "collection": "API", "path": path, "name": "All users" }),
        )
        .await["path"]
        .as_str()
        .unwrap()
        .to_owned();
    h.call(
        "rename",
        json!({ "collection": "API", "path": admin, "name": "Administration" }),
    )
    .await;
    let moved = h
        .call(
            "move",
            json!({ "collection": "API", "path": renamed, "folder": admin }),
        )
        .await["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let copy = h
        .call("duplicate", json!({ "collection": "API", "path": moved }))
        .await["path"]
        .as_str()
        .unwrap()
        .to_owned();
    h.call(
        "reorder",
        json!({ "collection": "API", "path": copy, "order": 1 }),
    )
    .await;
    h.call("delete", json!({ "collection": "API", "path": users }))
        .await;

    let tree = collection::load(&root).unwrap();
    let json = serde_json::to_value(&tree).unwrap();
    assert_eq!(
        json["children"].as_array().unwrap().len(),
        1,
        "Users is gone"
    );
    let folder = &json["children"][0];
    assert_eq!(folder["name"], "Administration");
    assert_eq!(
        folder["children"][0]["path"],
        copy.as_str(),
        "ordered first"
    );
    assert_eq!(folder["children"][1]["name"], "All users");
    assert!(moved.starts_with(&admin));
}

#[tokio::test]
async fn literal_secret_values_are_refused_everywhere_they_could_be_written() {
    let h = harness(&["API"]).await;
    let root = h.root(0);
    let env = environments::create(
        &root,
        &Environment {
            name: "Dev".into(),
            protected: false,
            variables: Default::default(),
            secrets: vec![SecretName {
                name: "token".into(),
                description: String::new(),
            }],
        },
    )
    .unwrap();
    h.secrets
        .set(
            &collection::id(&root).unwrap(),
            &env,
            "token",
            "sentinel-41ab",
        )
        .unwrap();
    let path = create(&h, "", "Me").await;

    let in_request = h
        .fails(
            "update_request",
            json!({ "collection": "API", "path": path,
                    "request": { "request": { "url": "https://x/?k=sentinel-41ab" } } }),
        )
        .await;
    let in_headers = h
        .fails(
            "set_headers",
            json!({ "collection": "API", "headers": [{ "name": "X-Key", "value": "sentinel-41ab" }] }),
        )
        .await;
    let in_auth = h
        .fails(
            "set_auth",
            json!({ "collection": "API", "auth": { "type": "bearer", "token": "sentinel-41ab" } }),
        )
        .await;
    let ok = h
        .call(
            "set_auth",
            json!({ "collection": "API", "auth": { "type": "bearer", "token": "{{token}}" } }),
        )
        .await;
    let headers = h
        .call(
            "set_headers",
            json!({ "collection": "API", "headers": [{ "name": "X-Tenant", "value": "acme" }] }),
        )
        .await;

    assert_eq!([in_request, in_headers, in_auth], ["contains_secret"; 3]);
    assert_eq!(ok["auth"]["token"], "{{token}}");
    assert_eq!(headers["headers"][0]["enabled"], true);
    let on_disk = walk(&root);
    assert!(!on_disk.contains("sentinel-41ab"));
}

#[tokio::test]
async fn malformed_requests_are_refused_before_any_file_is_written() {
    let h = harness(&["API"]).await;
    let root = h.root(0);

    let not_object = h
        .fails(
            "create_request",
            json!({ "collection": "API", "request": "GET /" }),
        )
        .await;
    let bad_body = h
        .fails(
            "create_request",
            json!({ "collection": "API", "request": { "request": { "body": { "type": "nope" } } } }),
        )
        .await;
    let bad_auth = h
        .fails(
            "set_auth",
            json!({ "collection": "API", "auth": { "type": "magic" } }),
        )
        .await;

    assert_eq!([not_object, bad_body, bad_auth], ["invalid_argument"; 3]);
    assert!(collection::load(&root).unwrap().children.is_empty());
}

/// Every file under `root`, concatenated.
pub fn walk(root: &std::path::Path) -> String {
    let mut out = String::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.push_str(&walk(&path));
        } else {
            out.push_str(&String::from_utf8_lossy(&std::fs::read(&path).unwrap()));
        }
    }
    out
}
