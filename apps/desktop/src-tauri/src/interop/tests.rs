#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use super::*;
use crate::ipc::CollectionCreateArgs;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../crates/wbl-interop/tests/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn a_postman_collection_is_summarised_imported_and_exported_as_http() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let collections = Collections::default();
    collections
        .create(&CollectionCreateArgs {
            root: root.clone(),
            name: "Mine".into(),
        })
        .unwrap();

    let seen = summary(&fixture("postman.json")).unwrap();
    let done = import(
        &collections,
        &InteropImportArgs {
            path: fixture("postman.json"),
            target: ImportTarget::Existing {
                root: root.clone(),
                folder_name: "Shop".into(),
            },
        },
    )
    .unwrap();
    let out = dir.path().join("shop.http");
    let count = export(
        &collections,
        &HttpExportArgs {
            root: root.clone(),
            folder: done.folder.clone(),
            destination: out.to_string_lossy().into_owned(),
        },
    )
    .unwrap();

    assert_eq!((seen.name.as_str(), seen.requests), ("Shop", 6));
    assert_eq!(seen.folders, ["Orders"]);
    assert_eq!(done.installed.requests.len(), 6);
    assert_eq!(count, 6);
    let text = std::fs::read_to_string(out).unwrap();
    assert!(
        text.contains("### List orders\nGET {{baseUrl}}/orders"),
        "{text}"
    );
    assert!(
        text.contains("Authorization: Bearer {{bearerToken}}"),
        "the folder's auth is inherited: {text}"
    );
}

#[test]
fn an_environment_file_adds_only_its_environment() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let collections = Collections::default();
    collections
        .create(&CollectionCreateArgs {
            root: root.clone(),
            name: "Mine".into(),
        })
        .unwrap();

    let done = import(
        &collections,
        &InteropImportArgs {
            path: fixture("environment.json"),
            target: ImportTarget::Existing {
                root,
                folder_name: "Staging".into(),
            },
        },
    )
    .unwrap();

    assert_eq!(done.folder, "");
    assert_eq!(done.installed.environments, ["staging"]);
    assert!(summary("/no/such/file.json").unwrap_err().code == "io");
}
