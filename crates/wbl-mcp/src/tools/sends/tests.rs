#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use wbl_format::collection;

use crate::test_support::{Harness, TOKEN, harness, server};

const SENTINEL: &str = "sentinel-b7f0";

async fn env(h: &Harness, name: &str, protected: bool) -> String {
    let id = h
        .call(
            "create_environment",
            json!({ "collection": "API", "name": name, "protected": protected,
                    "secrets": ["key", "token"] }),
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    h.secrets
        .set(&collection::id(&h.root(0)).unwrap(), &id, "key", SENTINEL)
        .unwrap();
    id
}

async fn request(h: &Harness, folder: &str, request: Value) -> String {
    h.call(
        "create_request",
        json!({ "collection": "API", "folder": folder, "request": request }),
    )
    .await["path"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn a_folder_runs_login_then_an_authorised_call_without_showing_the_token() {
    let h = harness(&["API"]).await;
    let base = server().await;
    let env = env(&h, "Dev", false).await;
    let flow = h
        .call(
            "create_folder",
            json!({ "collection": "API", "name": "Flow" }),
        )
        .await["path"]
        .as_str()
        .unwrap()
        .to_owned();
    request(
        &h,
        &flow,
        json!({ "name": "Login", "order": 10,
                "request": { "method": "POST", "url": format!("{base}/login") },
                "captures": [{ "var": "token", "from": "body", "path": "$.token", "regex": null, "secret": true }] }),
    )
    .await;
    request(
        &h,
        &flow,
        json!({ "name": "Me", "order": 20,
                "request": { "url": format!("{base}/me"),
                             "headers": [{ "name": "Authorization", "value": "Bearer {{token}}" }] },
                "assertions": [{ "check": "status", "equals": 200 }] }),
    )
    .await;

    let run = h
        .call(
            "run_folder",
            json!({ "collection": "API", "folder": flow, "environment": env }),
        )
        .await;

    assert_eq!(run["summary"]["passed"], 2, "{run}");
    assert_eq!(run["steps"][1]["status"], 200);
    assert!(!run.to_string().contains(TOKEN));
}

#[tokio::test]
async fn a_sent_request_comes_back_masked() {
    let h = harness(&["API"]).await;
    let base = server().await;
    let env = env(&h, "Dev", false).await;
    let path = request(
        &h,
        "",
        json!({ "name": "Echo", "request": { "url": format!("{base}/echo?k={{{{key}}}}"),
                "headers": [{ "name": "X-Key", "value": "{{key}}" }] } }),
    )
    .await;

    let sent = h
        .call(
            "send_request",
            json!({ "collection": "API", "path": path, "environment": env }),
        )
        .await;

    assert_eq!(sent["response"]["status"], 200);
    assert!(
        sent["response"]["body"]["text"]
            .as_str()
            .unwrap()
            .contains("x-key")
    );
    assert!(!sent.to_string().contains(SENTINEL), "{sent}");
}

#[tokio::test]
async fn protected_environments_unknown_examples_and_missing_values_are_refused() {
    let h = harness(&["API"]).await;
    let base = server().await;
    let prod = env(&h, "Prod", true).await;
    let path = request(
        &h,
        "",
        json!({ "name": "Me", "request": { "url": format!("{base}/me?u={{{{user}}}}") } }),
    )
    .await;

    let protected = h
        .fails(
            "send_request",
            json!({ "collection": "API", "path": path, "environment": prod }),
        )
        .await;
    let protected_run = h
        .fails(
            "run_folder",
            json!({ "collection": "API", "environment": prod }),
        )
        .await;
    let example = h
        .fails(
            "send_request",
            json!({ "collection": "API", "path": path, "example": "nope" }),
        )
        .await;
    let unresolved = h
        .call_raw("send_request", json!({ "collection": "API", "path": path }))
        .await;

    assert_eq!(protected, "protected_environment");
    assert_eq!(protected_run, "protected_environment");
    assert_eq!(example, "not_found");
    let error = unresolved.structured_content.unwrap();
    assert_eq!(error["code"], "unresolved");
    assert_eq!(error["detail"]["undefined"], json!(["user"]));
}
