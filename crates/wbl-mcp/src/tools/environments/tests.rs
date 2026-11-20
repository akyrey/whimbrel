#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::json;
use wbl_format::collection;

use crate::test_support::harness;
use crate::tools::requests::tests::walk;

const SENTINEL: &str = "sentinel-5e0c";

#[tokio::test]
async fn an_environment_is_created_changed_and_deleted_with_secrets_as_names() {
    let h = harness(&["API"]).await;
    let root = h.root(0);
    let created = h
        .call(
            "create_environment",
            json!({ "collection": "API", "name": "Staging",
                    "variables": { "base": "https://staging", "user": "ada" },
                    "secrets": ["token"] }),
        )
        .await;
    let id = created["id"].as_str().unwrap().to_owned();
    h.secrets
        .set(&collection::id(&root).unwrap(), &id, "token", SENTINEL)
        .unwrap();

    let changed = h
        .call(
            "set_variables",
            json!({ "collection": "API", "id": id,
                    "set": { "base": "https://stage2", "region": "eu" },
                    "secret": ["user"], "remove": ["region-missing"] }),
        )
        .await;
    let on_disk = walk(&root);
    let removed = h
        .call(
            "set_variables",
            json!({ "collection": "API", "id": id, "remove": ["token"] }),
        )
        .await;
    let deleted = h
        .call(
            "delete_environment",
            json!({ "collection": "API", "id": id }),
        )
        .await;

    assert_eq!(
        created["secrets"][0],
        json!({"name": "token", "description": "", "set": false})
    );
    assert_eq!(
        changed["variables"],
        json!({ "base": "https://stage2", "region": "eu" })
    );
    let names: Vec<&str> = changed["secrets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["token", "user"]);
    assert_eq!(changed["secrets"][0]["set"], true);
    assert_eq!(removed["secrets"].as_array().unwrap().len(), 1);
    assert_eq!(deleted["deleted"], id.as_str());
    assert!(
        on_disk.contains("stage2") && !on_disk.contains("ada"),
        "a value made secret leaves the file"
    );
    assert_eq!(
        h.secrets
            .get(&collection::id(&root).unwrap(), &id, "token")
            .unwrap()
            .as_deref(),
        Some(SENTINEL),
        "deleting the file keeps the keychain value"
    );
}

#[tokio::test]
async fn collection_variables_are_set_and_removed() {
    let h = harness(&["API"]).await;

    h.call(
        "set_collection_variables",
        json!({ "collection": "API", "set": { "base": "http://localhost", "v": "1" } }),
    )
    .await;
    let after = h
        .call(
            "set_collection_variables",
            json!({ "collection": "API", "remove": ["v"] }),
        )
        .await;

    assert_eq!(after["variables"], json!({ "base": "http://localhost" }));
}

#[tokio::test]
async fn no_secret_value_reaches_disk_or_a_result_through_environment_tools() {
    let h = harness(&["API"]).await;
    let root = h.root(0);
    let id = h
        .call(
            "create_environment",
            json!({ "collection": "API", "name": "Dev", "secrets": ["key"] }),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    h.secrets
        .set(&collection::id(&root).unwrap(), &id, "key", SENTINEL)
        .unwrap();

    let codes = [
        h.fails(
            "set_variables",
            json!({ "collection": "API", "id": id, "set": { "copy": SENTINEL } }),
        )
        .await,
        h.fails(
            "set_collection_variables",
            json!({ "collection": "API", "set": { "copy": format!("x{SENTINEL}") } }),
        )
        .await,
        h.fails(
            "create_environment",
            json!({ "collection": "API", "name": "Leak", "variables": { "k": SENTINEL } }),
        )
        .await,
    ];
    let shown = h
        .call("get_environment", json!({ "collection": "API", "id": id }))
        .await;

    assert_eq!(codes, ["contains_secret"; 3]);
    assert!(!shown.to_string().contains(SENTINEL));
    assert!(!walk(&root).contains(SENTINEL));
}
