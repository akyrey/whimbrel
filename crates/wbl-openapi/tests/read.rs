#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;

use serde_json::json;
use url::Url;
use wbl_openapi::{Loader, OpenApiError, load_all, parse};

struct Files(HashMap<String, String>);

impl Loader for Files {
    async fn load(&self, url: &Url) -> Result<String, OpenApiError> {
        self.0
            .get(url.as_str())
            .cloned()
            .ok_or_else(|| OpenApiError::Load(format!("no {url}")))
    }
}

#[test]
fn yaml_and_json_read_alike() {
    let yaml = "openapi: 3.1.0\ninfo:\n  title: Pets\n  version: 1.0\npaths:\n  /pets:\n    get:\n      responses:\n        200:\n          description: ok\n        default: {description: err}\nx-flag: true\nx-ratio: 0.5\nx-none: ~\n";
    let json_text = r#"{"openapi":"3.1.0","info":{"title":"Pets","version":1.0}}"#;

    let value = parse(yaml).unwrap();

    assert_eq!(value["info"]["version"], json!(1.0));
    assert!(
        value["paths"]["/pets"]["get"]["responses"]["200"].is_object(),
        "integer keys become text"
    );
    assert_eq!(value["x-flag"], json!(true));
    assert_eq!(value["x-ratio"], json!(0.5));
    assert!(value["x-none"].is_null());
    assert_eq!(parse(json_text).unwrap()["info"]["title"], "Pets");
    assert!(matches!(
        parse("{ nope").unwrap_err(),
        OpenApiError::Syntax(_)
    ));
    assert!(matches!(
        parse("a: [").unwrap_err(),
        OpenApiError::Syntax(_)
    ));
}

#[test]
fn yaml_anchors_are_followed() {
    let value = parse("base: &b {type: string}\ncopy: *b\n").unwrap();
    assert_eq!(value["copy"]["type"], "string");
}

#[tokio::test]
async fn loads_referenced_files_once_and_remote_ones_only_when_allowed() {
    let entry = Url::parse("file:///specs/api.yaml").unwrap();
    let files = Files(HashMap::from([
        (
            "file:///specs/schemas/pet.yaml".to_owned(),
            "type: object\nproperties:\n  owner: {$ref: './owner.json'}\n".to_owned(),
        ),
        (
            "file:///specs/schemas/owner.json".to_owned(),
            r#"{"type":"object"}"#.to_owned(),
        ),
    ]));
    let text = "openapi: 3.0.3\ncomponents:\n  schemas:\n    Pet: {$ref: 'schemas/pet.yaml'}\n    Again: {$ref: 'schemas/pet.yaml#/properties'}\n    Remote: {$ref: 'https://example.com/x.json'}\n";

    let loaded = load_all(entry.clone(), text, &files, false).await.unwrap();
    let docs = &loaded.documents;

    assert_eq!(docs.by_url.len(), 3);
    assert_eq!(
        docs.get(&Url::parse("file:///specs/schemas/owner.json").unwrap())
            .unwrap()["type"],
        "object"
    );
    assert_eq!(docs.entry_value().unwrap()["openapi"], "3.0.3");
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.remote, ["https://example.com/x.json"]);

    let warnings = load_all(entry, text, &files, true).await.unwrap().warnings;
    assert!(
        warnings[0].starts_with("Could not read https://example.com/x.json"),
        "{warnings:?}"
    );
}

#[tokio::test]
async fn self_sets_the_base_of_references() {
    let entry = Url::parse("file:///local/copy.yaml").unwrap();
    let files = Files(HashMap::from([(
        "file:///published/v1/schemas.yaml".to_owned(),
        "Pet: {type: object}\n".to_owned(),
    )]));
    let text = "openapi: 3.2.0\n$self: file:///published/v1/api.yaml\ncomponents:\n  schemas:\n    Pet: {$ref: 'schemas.yaml#/Pet'}\n";

    let loaded = load_all(entry, text, &files, false).await.unwrap();

    assert_eq!(
        loaded.documents.entry.as_ref().unwrap().as_str(),
        "file:///published/v1/api.yaml"
    );
    assert!(
        loaded
            .documents
            .by_url
            .contains_key("file:///published/v1/schemas.yaml")
    );
    assert!(
        loaded
            .documents
            .by_url
            .contains_key("file:///local/copy.yaml"),
        "still found where it was read"
    );
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
}
