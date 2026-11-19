#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 6.8: a 5 MB document imports (read, model, generate, write) in under 3 seconds.

mod support;

use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};
use url::Url;
use wbl_openapi::{Source, build, generate, install, load_all};

const TARGET_BYTES: usize = 5 * 1024 * 1024;
const LIMIT: Duration = Duration::from_secs(3);

fn schema(i: usize) -> Value {
    let properties: Map<String, Value> = (0..12)
        .map(|p| (format!("field{p}"), json!({"type": "string", "description": format!("Field {p} of resource {i}, described at some length so the document has realistic weight."), "maxLength": 200})))
        .chain([("next".to_owned(), json!({"$ref": format!("#/components/schemas/Resource{}", (i + 1) % 400)}))])
        .collect();
    json!({"type": "object", "required": ["field0"], "properties": properties})
}

fn operation(i: usize, method: &str) -> Value {
    json!({
        "operationId": format!("{method}Resource{i}"),
        "summary": format!("{method} resource {i}"),
        "tags": [format!("group{}", i % 40)],
        "parameters": [
            {"name": "id", "in": "path", "required": true, "schema": {"type": "integer"}},
            {"name": "expand", "in": "query", "schema": {"type": "array", "items": {"type": "string", "enum": ["a", "b"]}}},
            {"name": "X-Request-Id", "in": "header", "schema": {"type": "string", "format": "uuid"}}
        ],
        "requestBody": {"content": {"application/json": {"schema": {"$ref": format!("#/components/schemas/Resource{}", i % 400)}}}},
        "responses": {
            "200": {"description": "ok", "content": {"application/json": {"schema": {"$ref": format!("#/components/schemas/Resource{}", i % 400)}}}},
            "404": {"description": "not found"}
        }
    })
}

fn large_document() -> String {
    let schemas: Map<String, Value> = (0..400)
        .map(|i| (format!("Resource{i}"), schema(i)))
        .collect();
    let mut paths = Map::new();
    let mut i = 0;
    let mut doc = json!({"openapi": "3.1.0", "info": {"title": "Large", "version": "1"}, "servers": [{"url": "https://large.example.com"}], "components": {"schemas": schemas}});
    while serde_json::to_string_pretty(&doc).unwrap().len() < TARGET_BYTES {
        for _ in 0..50 {
            paths.insert(
                format!("/resources{i}/{{id}}"),
                json!({"get": operation(i, "get"), "put": operation(i, "put")}),
            );
            i += 1;
        }
        doc["paths"] = Value::Object(paths.clone());
    }
    serde_json::to_string_pretty(&doc).unwrap()
}

#[test]
fn a_5_mb_document_imports_in_under_3_seconds() {
    let text = large_document();
    let dir = tempfile::tempdir().unwrap();
    wbl_format::collection::create(dir.path(), "Large").unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let started = Instant::now();
    let url = Url::parse("file:///large.json").unwrap();
    let docs = runtime
        .block_on(load_all(url, &text, &support::Disk, false))
        .unwrap()
        .documents;
    let read = started.elapsed();
    let api = build(docs).unwrap();
    let built = started.elapsed();
    let generated = generate(&api);
    let made = started.elapsed();
    let installed = install(
        dir.path(),
        "",
        &generated,
        &Source {
            location: "large.json".into(),
            text: text.clone(),
        },
    )
    .unwrap();
    let elapsed = started.elapsed();

    println!(
        "{} bytes, {} requests in {elapsed:?} (read {read:?}, model {built:?}, generated {made:?})",
        text.len(),
        installed.requests.len()
    );
    assert!(text.len() >= TARGET_BYTES);
    assert_eq!(installed.requests.len(), api.operations.len());
    assert!(elapsed < LIMIT, "took {elapsed:?}");
}
