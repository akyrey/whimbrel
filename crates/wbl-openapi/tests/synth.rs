#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 6.4: example values from schemas.

mod support;

use std::collections::BTreeMap;

use serde_json::{Value, json};
use wbl_openapi::synth::{Direction, example};

fn of(schema: Value) -> Value {
    example(&schema, &BTreeMap::new(), Direction::Request)
}

#[test]
fn stated_values_come_first() {
    assert_eq!(
        of(json!({"type": "string", "const": "x", "default": "d"})),
        json!("x")
    );
    assert_eq!(
        of(json!({"type": "integer", "examples": [3, 4], "default": 1})),
        json!(3)
    );
    assert_eq!(
        of(json!({"type": "integer", "default": 1, "enum": [5]})),
        json!(1)
    );
    assert_eq!(
        of(json!({"type": "string", "enum": ["b", "a"]})),
        json!("b")
    );
    assert_eq!(
        of(json!({"type": "string", "example": "legacy"})),
        json!("legacy")
    );
}

#[test]
fn types_and_formats() {
    assert_eq!(
        of(json!({"type": "string", "format": "uuid"})),
        json!("3fa85f64-5717-4562-b3fc-2c963f66afa6")
    );
    assert_eq!(
        of(json!({"type": "string", "format": "date-time"})),
        json!("2024-01-01T12:00:00Z")
    );
    assert_eq!(
        of(json!({"type": "string", "minLength": 8})),
        json!("stringxx")
    );
    assert_eq!(of(json!({"type": "integer", "minimum": 5})), json!(5));
    assert_eq!(
        of(json!({"type": "integer", "exclusiveMinimum": 5})),
        json!(6)
    );
    assert_eq!(of(json!({"type": "number"})), json!(0));
    assert_eq!(of(json!({"type": "boolean"})), json!(true));
    assert_eq!(of(json!({"type": ["null", "string"]})), json!("string"));
    assert_eq!(of(json!({"type": "null"})), Value::Null);
    assert_eq!(
        of(json!({"type": "array", "prefixItems": [{"type": "number"}, {"type": "string"}]})),
        json!([0, "string"])
    );
    assert_eq!(
        of(json!({"type": "array", "items": {"type": "integer"}})),
        json!([0])
    );
    assert_eq!(
        of(json!({"type": "object", "additionalProperties": {"type": "integer"}})),
        json!({"key": 0})
    );
    assert_eq!(of(json!(true)), json!({}));
}

#[test]
fn combinations_and_read_only() {
    let schema = json!({
        "allOf": [
            {"type": "object", "properties": {"a": {"type": "integer"}}},
            {"type": "object", "properties": {"b": {"type": "string", "readOnly": true}, "c": {"type": "boolean"}}}
        ]
    });
    assert_eq!(of(schema.clone()), json!({"a": 0, "c": true}));
    assert_eq!(
        example(&schema, &BTreeMap::new(), Direction::Response),
        json!({"a": 0, "b": "string", "c": true})
    );
    assert_eq!(
        of(json!({"oneOf": [{"type": "string", "format": "email"}, {"type": "integer"}]})),
        json!("user@example.com")
    );
    assert_eq!(of(json!({"anyOf": [{"type": "integer"}]})), json!(0));
}

#[test]
fn cycles_stop_and_references_follow() {
    let api = support::api("openapi30.yaml");
    let create = api
        .operations
        .iter()
        .find(|o| o.id == "createUser")
        .unwrap();
    let schema = create.request_body.as_ref().unwrap().content[0]
        .schema
        .clone()
        .unwrap();

    let value = example(&schema, &api.schemas, Direction::Request);

    // NewUser → Team → members: [User = NewUser (a cycle: left out) + its own fields].
    assert_eq!(value["name"], "Ada");
    assert_eq!(value["email"], "user@example.com");
    assert_eq!(value["role"], "member");
    assert_eq!(
        value["team"],
        json!({"name": "string", "members": [{"id": 0, "nickname": "string"}]})
    );
}

#[test]
fn self_reference_and_siblings() {
    let api = support::api("openapi31.yaml");
    let event = example(
        &json!({"$ref": "#/schemas/Event"}),
        &api.schemas,
        Direction::Response,
    );

    assert_eq!(event["kind"], "created");
    assert_eq!(event["point"], json!([0, 0]));
    assert_eq!(event["payload"], json!({"size": 7}));
    assert_eq!(event["level"], 3);
    assert!(
        event.get("parent").is_none(),
        "an optional self-reference is left out"
    );
    assert_eq!(event["note"], "string");
}
