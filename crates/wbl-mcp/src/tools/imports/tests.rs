#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use serde_json::json;

use crate::test_support::harness;

fn fixture(krate: &str, name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(krate)
        .join("tests/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[tokio::test]
async fn an_openapi_file_is_imported_into_a_new_folder() {
    let h = harness(&["API"]).await;

    let done = h
        .call(
            "import_openapi",
            json!({ "collection": "API", "source": fixture("wbl-openapi", "openapi30.yaml"),
                    "folder_name": "Users API" }),
        )
        .await;
    let tree = h.call("get_tree", json!({ "collection": "API" })).await;

    assert_eq!(done["summary"]["title"], "Users");
    assert_eq!(done["installed"]["requests"].as_object().unwrap().len(), 3);
    assert_eq!(tree["children"][0]["name"], "Users API");
    let envs = h
        .call("list_environments", json!({ "collection": "API" }))
        .await;
    assert_eq!(envs["environments"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn a_postman_collection_is_imported_and_exported_as_http() {
    let h = harness(&["API"]).await;

    let done = h
        .call(
            "import_file",
            json!({ "collection": "API", "path": fixture("wbl-interop", "postman.json"),
                    "folder_name": "Shop" }),
        )
        .await;
    let exported = h
        .call(
            "export_http",
            json!({ "collection": "API", "folder": done["folder"] }),
        )
        .await;

    let count = done["installed"]["requests"].as_array().unwrap().len();
    assert!(count > 0);
    assert_eq!(exported["requests"], count);
    assert!(!exported["text"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn a_curl_command_becomes_a_request() {
    let h = harness(&["API"]).await;

    let done = h
        .call(
            "import_curl",
            json!({ "collection": "API",
                    "command": "curl -X POST https://api.test/users -H 'Content-Type: application/json' -d '{\"a\":1}' --compressed" }),
        )
        .await;
    let bad = h
        .fails(
            "import_curl",
            json!({ "collection": "API", "command": "wget x" }),
        )
        .await;
    let missing = h
        .fails(
            "import_openapi",
            json!({ "collection": "API", "source": "/no/such.yaml", "folder_name": "X" }),
        )
        .await;

    assert_eq!(done["request"]["name"], "POST https://api.test/users");
    assert_eq!(done["request"]["request"]["method"], "POST");
    assert!(done["path"].as_str().unwrap().ends_with(".req.toml"));
    assert_eq!(bad, "invalid_curl");
    assert_eq!(missing, "not_found");
}
