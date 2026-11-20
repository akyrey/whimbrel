#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use super::*;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../wbl-interop/tests/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn a_postman_collection_is_summarised_installed_and_exported() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    collection::create(&root, "Mine").unwrap();
    let seen = summary(&fixture("postman.json")).unwrap();
    let imported = read(&fixture("postman.json")).unwrap();
    let folder = collection::create_folder(&root, "", "Shop").unwrap();
    wbl_interop::install(&root, &folder, &imported).unwrap();

    let (all, count) = export(&root, "").unwrap();
    let (none, zero) = export(&root, "elsewhere").unwrap();

    assert!(seen.requests > 0);
    assert_eq!(count, seen.requests);
    assert!(all.contains("###") || all.contains("HTTP/1.1") || all.contains("GET "));
    assert_eq!(zero, 0);
    assert!(none.trim().is_empty());
}

#[test]
fn an_unreadable_file_says_why() {
    assert_eq!(summary("/no/such/file.json").unwrap_err().code, "io");
}
