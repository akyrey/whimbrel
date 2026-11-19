#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 7.5 and the Phase 7 acceptance: re-importing a changed spec keeps user edits.

use std::collections::BTreeMap;
use std::path::Path;

use url::Url;
use wbl_format::collection;
use wbl_openapi::reimport::{apply, plan};
use wbl_openapi::{Api, Change, Choice, Field, Source, build, generate, install, parse};

const V1: &str = r#"
openapi: 3.1.0
info: {title: Users, version: "1"}
paths:
  /users:
    get:
      tags: [users]
      operationId: listUsers
      summary: List users
      parameters: [{name: limit, in: query, schema: {type: integer}}]
      responses: {"200": {description: ok}}
  /users/{id}:
    get:
      tags: [users]
      operationId: getUser
      summary: Get a user
      parameters: [{name: id, in: path, required: true, schema: {type: integer}}]
      responses: {"200": {description: ok}}
    delete:
      tags: [users]
      operationId: deleteUser
      summary: Delete a user
      parameters: [{name: id, in: path, required: true, schema: {type: integer}}]
      responses: {"204": {description: gone}}
"#;

const V2: &str = r#"
openapi: 3.1.0
info: {title: Users, version: "2"}
paths:
  /users:
    get:
      tags: [users]
      operationId: listUsers
      summary: List users
      parameters:
        - {name: limit, in: query, schema: {type: integer}}
        - {name: role, in: query, required: true, schema: {type: string, enum: [admin]}}
      responses: {"200": {description: ok}}
    post:
      tags: [users]
      operationId: createUser
      summary: Create a user
      responses: {"201": {description: created}}
  /users/{id}:
    get:
      tags: [users]
      operationId: getUser
      summary: Fetch one user
      parameters:
        - {name: id, in: path, required: true, schema: {type: integer}}
        - {name: X-Trace, in: header, required: true, schema: {type: string}}
      responses: {"200": {description: ok}}
  /teams:
    get:
      tags: [teams]
      operationId: listTeams
      responses: {"200": {description: ok}}
"#;

fn api(text: &str) -> Api {
    let documents = wbl_openapi::Documents {
        entry: Some(Url::parse("file:///users.yaml").unwrap()),
        by_url: [("file:///users.yaml".to_owned(), parse(text).unwrap())].into(),
    };
    build(documents).unwrap()
}

fn source(text: &str) -> Source {
    Source {
        location: "users.yaml".into(),
        text: text.into(),
    }
}

/// v1 imported, then the user renames getUser and adds a query parameter to its URL.
fn imported() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "Users").unwrap();
    let installed = install(root, "", &generate(&api(V1)), &source(V1)).unwrap();
    let path = &installed.requests["getUser"];
    let read = collection::read_request(root, path).unwrap();
    let edited = wbl_format::StoredRequest {
        name: "My user".into(),
        request: wbl_core::Request {
            url: format!("{}?expand=team", read.stored.request.url),
            ..read.stored.request.clone()
        },
        ..read.stored
    };
    collection::save_request(root, path, &edited, &read.hash).unwrap();
    (dir, installed.spec_dir)
}

fn kinds(changes: &[Change]) -> Vec<(String, String)> {
    changes
        .iter()
        .map(|c| match c {
            Change::Added { operation_id, .. } => ("added".into(), operation_id.clone()),
            Change::Removed { operation_id, .. } => ("removed".into(), operation_id.clone()),
            Change::Updated { operation_id, .. } => ("updated".into(), operation_id.clone()),
            Change::Merged { operation_id, .. } => ("merged".into(), operation_id.clone()),
        })
        .collect()
}

fn read(root: &Path, path: &str) -> wbl_format::StoredRequest {
    collection::read_request(root, path).unwrap().stored
}

#[test]
fn the_plan_names_what_changes() {
    let (dir, spec) = imported();

    let planned = plan(dir.path(), &spec, &api(V1), &api(V2)).unwrap();

    let mut got = kinds(&planned.changes);
    got.sort();
    assert_eq!(
        got,
        [
            ("added".into(), "createUser".into()),
            ("added".into(), "listTeams".into()),
            ("merged".into(), "getUser".into()),
            ("removed".into(), "deleteUser".into()),
            ("updated".into(), "listUsers".into()),
        ]
    );
    let Some(Change::Merged { conflicts, .. }) = planned
        .changes
        .iter()
        .find(|c| matches!(c, Change::Merged { .. }))
    else {
        panic!()
    };
    assert_eq!(conflicts.len(), 1, "{conflicts:?}");
    assert_eq!(conflicts[0].field, Field::Name);
    assert_eq!(conflicts[0].current, "My user");
    assert_eq!(conflicts[0].new, "Fetch one user");
}

#[test]
fn applying_keeps_the_users_edits_and_takes_the_rest() {
    let (dir, spec) = imported();
    let root = dir.path();

    apply(
        root,
        &spec,
        &api(V1),
        &api(V2),
        &source(V2),
        &BTreeMap::new(),
    )
    .unwrap();

    let kept = wbl_openapi::specs(root).unwrap().remove(0);
    let get = read(root, &kept.operations["getUser"]);
    assert_eq!(
        get.name, "My user",
        "the conflict keeps the user's side by default"
    );
    assert!(
        get.request.url.ends_with("?expand=team"),
        "the user's URL stays: the spec did not change it"
    );
    assert_eq!(
        get.request.headers[0].name, "X-Trace",
        "the spec's new header comes in"
    );
    let list = read(root, &kept.operations["listUsers"]);
    assert_eq!(list.request.url, "{{baseUrl}}/users?role=admin");
    assert!(
        kept.operations["listTeams"].starts_with("teams/"),
        "a new tag makes a new folder"
    );
    assert!(read(root, &kept.operations["createUser"]).request.method == "POST");
    let deleted = std::fs::read_to_string(root.join(&kept.operations["deleteUser"])).unwrap();
    assert!(deleted.contains("removed = true"), "{deleted}");
    assert!(deleted.contains("operationId = \"deleteUser\""));
    assert_eq!(
        std::fs::read_to_string(root.join(&spec).join("source.yaml")).unwrap(),
        V2
    );
    let file = std::fs::read_to_string(root.join(&kept.operations["getUser"])).unwrap();
    assert!(file.contains("[openapi]"), "{file}");

    let again = plan(root, &spec, &api(V2), &api(V2)).unwrap();
    assert!(
        again.changes.is_empty(),
        "nothing left to do: {:?}",
        again.changes
    );
}

#[test]
fn a_conflict_can_take_the_specs_side() {
    let (dir, spec) = imported();
    let root = dir.path();
    let choices = BTreeMap::from([(
        "getUser".to_owned(),
        BTreeMap::from([(Field::Name, Choice::Theirs)]),
    )]);

    apply(root, &spec, &api(V1), &api(V2), &source(V2), &choices).unwrap();

    let kept = wbl_openapi::specs(root).unwrap().remove(0);
    let get = read(root, &kept.operations["getUser"]);
    assert_eq!(get.name, "Fetch one user");
    assert!(get.request.url.ends_with("?expand=team"));
}

#[test]
fn requests_deleted_by_hand_are_left_alone() {
    let (dir, spec) = imported();
    let root = dir.path();
    let kept = wbl_openapi::specs(root).unwrap().remove(0);
    std::fs::remove_file(root.join(&kept.operations["listUsers"])).unwrap();

    let planned = plan(root, &spec, &api(V1), &api(V2)).unwrap();

    assert_eq!(planned.skipped, ["listUsers"]);
}
