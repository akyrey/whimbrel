#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 7.4: responses checked against their operation's schema.

mod support;

use wbl_openapi::validate;

#[test]
fn a_matching_body_passes_and_a_wrong_one_names_where() {
    let api = support::api("openapi30.yaml");
    let good = r#"{"id": 1, "name": "Ada", "email": "ada@example.com", "role": "admin", "nickname": null}"#;
    let bad = r#"{"id": "one", "email": "ada@example.com", "role": "boss"}"#;

    let ok = validate(
        &api,
        "getUser",
        200,
        Some("application/json; charset=utf-8"),
        good,
    );
    let wrong = validate(&api, "getUser", 200, Some("application/json"), bad);

    assert!(ok.checked && ok.errors.is_empty(), "{ok:?}");
    assert_eq!(ok.response.as_deref(), Some("200"));
    assert!(wrong.checked);
    let pointers: Vec<_> = wrong.errors.iter().map(|e| e.pointer.as_str()).collect();
    assert!(pointers.contains(&"/id"), "{wrong:?}");
    assert!(pointers.contains(&"/role"), "{wrong:?}");
    assert!(
        wrong.errors.iter().any(|e| e.message.contains("name")),
        "{wrong:?}"
    );
}

#[test]
fn ranges_default_and_cycles() {
    let api = support::api("openapi30.yaml");
    let error = validate(
        &api,
        "getUser",
        503,
        Some("application/json"),
        r#"{"message": 3}"#,
    );
    assert_eq!(error.response.as_deref(), Some("default"));
    assert_eq!(error.errors[0].pointer, "/message");

    let team = r#"[{"id": 1, "name": "A", "email": "a@x", "team": {"name": "t", "members": [{"id": 2, "name": "B", "email": "b@x"}]}}]"#;
    let list = validate(&api, "listUsers", 200, Some("application/json"), team);
    assert!(list.checked && list.errors.is_empty(), "{list:?}");
}

#[test]
fn what_cannot_be_checked_says_why() {
    let api = support::api("openapi30.yaml");
    let note = |v: wbl_openapi::Validation| v.note.unwrap_or_default();

    assert!(note(validate(&api, "nope", 200, None, "{}")).contains("no longer"));
    assert!(note(validate(&api, "listUsers", 404, None, "{}")).contains("404"));
    assert!(
        note(validate(
            &api,
            "getUser",
            200,
            Some("application/json"),
            "<html>"
        ))
        .contains("not JSON")
    );
    let swagger = support::api("swagger2.yaml");
    assert!(
        note(validate(
            &swagger,
            "addPet",
            405,
            Some("application/json"),
            "{}"
        ))
        .contains("no body")
    );
    let spec32 = support::api("openapi32.yaml");
    assert!(
        note(validate(
            &spec32,
            "searchQuery",
            200,
            Some("application/jsonl"),
            "{}"
        ))
        .contains("no schema")
    );
    assert!(
        note(validate(&api, "getUser", 200, Some("text/csv"), "{}")).is_empty(),
        "the only media type is used"
    );
}
