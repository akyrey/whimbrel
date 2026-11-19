#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use wbl_core::{Body, Request};
use wbl_format::collection::{self, Node};
use wbl_format::{FormatError, StoredRequest};

fn request(name: &str, order: i64) -> StoredRequest {
    StoredRequest {
        name: name.into(),
        order,
        request: Request {
            method: "GET".into(),
            url: "https://example.com".into(),
            headers: vec![],
            body: Body::None,
        },
        examples: vec![],
        settings: Default::default(),
        auth: Default::default(),
        variables: Default::default(),
    }
}

fn names(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Folder(f) => format!("{}/", f.name),
            Node::Request(r) => r.name.clone(),
            Node::Broken(b) => format!("!{}", b.path),
        })
        .collect()
}

fn new_collection(dir: &Path) {
    collection::create(dir, "My API").unwrap();
}

#[test]
fn creating_a_collection_writes_its_file_and_ignores_local_overrides() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());

    let loaded = collection::load(dir.path()).unwrap();

    assert_eq!(loaded.name, "My API");
    assert_eq!(loaded.id.len(), 36, "a uuid");
    assert!(loaded.children.is_empty());
    assert_eq!(
        fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
        ".local/\n"
    );
    assert!(
        collection::create(dir.path(), "Again").is_err(),
        "never overwrites one"
    );
}

#[test]
fn a_folder_without_a_collection_file_is_not_a_collection() {
    let dir = tempfile::tempdir().unwrap();
    let error = collection::load(dir.path()).unwrap_err();
    assert!(
        matches!(error, FormatError::Missing(ref f) if f == "collection.toml"),
        "{error}"
    );
}

#[test]
fn the_tree_is_ordered_by_order_then_name_with_folders_and_broken_files() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());
    let root = dir.path();
    let b = collection::create_request(root, "", &request("B request", 0)).unwrap();
    let a = collection::create_request(root, "", &request("A request", 0)).unwrap();
    let users = collection::create_folder(root, "", "Users").unwrap();
    collection::create_request(root, &users, &request("List users", 0)).unwrap();
    fs::write(root.join("broken.req.toml"), "name = ").unwrap();
    fs::create_dir(root.join(".git")).unwrap();
    fs::write(root.join(".git/x.req.toml"), "ignored").unwrap();
    fs::create_dir(root.join("environments")).unwrap();

    let loaded = collection::load(root).unwrap();

    assert_eq!(a, "a-request.req.toml");
    assert_eq!(b, "b-request.req.toml");
    // Created in order B, A, Users: orders 10, 20, 30; the broken file has none (0).
    assert_eq!(
        names(&loaded.children),
        vec!["!broken.req.toml", "B request", "A request", "Users/"]
    );
    let Node::Folder(folder) = &loaded.children[3] else {
        panic!()
    };
    assert_eq!(folder.path, "users");
    assert_eq!(names(&folder.children), vec!["List users"]);
    let Node::Broken(broken) = &loaded.children[0] else {
        panic!()
    };
    assert!(broken.error.contains("line 1"), "{}", broken.error);
}

#[test]
fn names_become_unique_slugs() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());
    let root = dir.path();

    let first = collection::create_request(root, "", &request("Get user #1 (by id)", 0)).unwrap();
    let second = collection::create_request(root, "", &request("Get user #1 (by id)", 0)).unwrap();
    let odd = collection::create_request(root, "", &request("¿Qué?", 0)).unwrap();

    assert_eq!(first, "get-user-1-by-id.req.toml");
    assert_eq!(second, "get-user-1-by-id-2.req.toml");
    assert_eq!(odd, "qu.req.toml");
}

#[test]
fn reading_and_saving_detects_changes_made_meanwhile() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());
    let root = dir.path();
    let path = collection::create_request(root, "", &request("Ping", 0)).unwrap();
    let read = collection::read_request(root, &path).unwrap();
    let edited = StoredRequest {
        request: Request {
            url: "https://example.org".into(),
            ..read.stored.request.clone()
        },
        ..read.stored.clone()
    };

    let saved = collection::save_request(root, &path, &edited, &read.hash).unwrap();
    let stale = collection::save_request(root, &path, &edited, &read.hash).unwrap_err();

    assert_ne!(saved, read.hash);
    assert_eq!(collection::read_request(root, &path).unwrap().hash, saved);
    assert_eq!(stale, FormatError::Conflict);
}

#[test]
fn a_broken_file_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());
    let root = dir.path();
    fs::write(root.join("broken.req.toml"), "name = ").unwrap();
    let hash = collection::hash(b"name = ");

    let error =
        collection::save_request(root, "broken.req.toml", &request("x", 0), &hash).unwrap_err();

    assert!(matches!(error, FormatError::Syntax(_)), "{error}");
    assert_eq!(
        fs::read_to_string(root.join("broken.req.toml")).unwrap(),
        "name = "
    );
}

#[test]
fn rename_duplicate_move_and_delete() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());
    let root = dir.path();
    let path = collection::create_request(root, "", &request("Ping", 0)).unwrap();
    let folder = collection::create_folder(root, "", "Health").unwrap();

    let renamed = collection::rename_request(root, &path, "Ping server").unwrap();
    let copy = collection::duplicate_request(root, &renamed).unwrap();
    let moved = collection::move_request(root, &copy, &folder).unwrap();

    assert_eq!(renamed, "ping-server.req.toml");
    assert!(!root.join(&path).exists());
    assert_eq!(moved, "health/ping-server-copy.req.toml");
    assert_eq!(
        collection::read_request(root, &moved).unwrap().stored.name,
        "Ping server copy"
    );
    collection::delete(root, &renamed).unwrap();
    collection::delete(root, &folder).unwrap();
    assert!(collection::load(root).unwrap().children.is_empty());
}

#[test]
fn paths_outside_the_collection_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());
    let root = dir.path();

    for bad in [
        "../x.req.toml",
        "/etc/passwd",
        "a/../../x",
        ".git/config",
        "",
        "collection.toml",
    ] {
        let error = collection::delete(root, bad).unwrap_err();
        assert!(matches!(error, FormatError::Path(_)), "{bad}: {error}");
    }
}

#[test]
fn reordering_moves_one_file() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());
    let root = dir.path();
    collection::create_request(root, "", &request("One", 0)).unwrap();
    collection::create_request(root, "", &request("Two", 0)).unwrap();
    let three = collection::create_request(root, "", &request("Three", 0)).unwrap();

    collection::set_order(root, &three, 15).unwrap();

    assert_eq!(
        names(&collection::load(root).unwrap().children),
        vec!["One", "Three", "Two"]
    );
}

#[test]
fn renaming_to_the_same_file_name_or_moving_to_the_same_folder_keeps_the_file() {
    let dir = tempfile::tempdir().unwrap();
    new_collection(dir.path());
    let root = dir.path();
    let path = collection::create_request(root, "", &request("Ping", 0)).unwrap();

    let renamed = collection::rename_request(root, &path, "PING").unwrap();
    let moved = collection::move_request(root, &renamed, "").unwrap();

    assert_eq!(renamed, "ping.req.toml");
    assert_eq!(moved, "ping.req.toml");
    assert_eq!(
        collection::read_request(root, &moved).unwrap().stored.name,
        "PING"
    );
}
