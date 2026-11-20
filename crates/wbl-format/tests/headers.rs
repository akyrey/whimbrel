#![allow(clippy::unwrap_used, clippy::expect_used)]

use wbl_core::Header;
use wbl_format::collection::{self, headers};

fn header(name: &str, value: &str) -> Header {
    Header {
        name: name.into(),
        value: value.into(),
        enabled: true,
    }
}

#[test]
fn folders_and_the_collection_hold_headers_that_requests_inherit() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let users = collection::create_folder(root, "", "Users").unwrap();
    let admin = collection::create_folder(root, &users, "Admin").unwrap();
    let common = vec![
        header("X-Tenant", "{{tenant}}"),
        Header {
            enabled: false,
            ..header("X-Debug", "1")
        },
    ];

    headers::save_headers(root, "", &common).unwrap();
    headers::save_headers(root, &users, &[header("X-Area", "users")]).unwrap();

    assert_eq!(headers::headers_of(root, "").unwrap(), common);
    let chain = headers::headers_chain(root, &format!("{admin}/list.req.toml")).unwrap();
    assert_eq!(
        chain,
        vec![vec![], vec![header("X-Area", "users")], common.clone()]
    );
    assert_eq!(
        headers::headers_chain(root, "ping.req.toml").unwrap(),
        vec![common]
    );
    assert_eq!(collection::load(root).unwrap().name, "API");
}

#[test]
fn no_headers_removes_the_list_and_a_folder_keeps_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let users = collection::create_folder(root, "", "Users").unwrap();

    headers::save_headers(root, &users, &[header("Accept", "application/json")]).unwrap();
    let written = std::fs::read_to_string(root.join("users/_folder.toml")).unwrap();
    assert!(written.contains("[[headers]]"), "{written}");
    headers::save_headers(root, &users, &[]).unwrap();

    let text = std::fs::read_to_string(root.join("users/_folder.toml")).unwrap();
    assert!(!text.contains("headers"), "{text}");
    assert!(text.contains("name = \"Users\""));
    assert!(headers::save_headers(root, "../x", &[]).is_err());
    assert!(headers::save_headers(root, "nowhere", &[]).is_err());
}

#[test]
fn saving_unchanged_headers_leaves_the_file_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let path = root.join("collection.toml");
    let text = std::fs::read_to_string(&path).unwrap()
        + "\n# shared by every request\n[[headers]]\nname = \"Accept\"\nvalue = \"application/json\"\n";
    std::fs::write(&path, &text).unwrap();

    headers::save_headers(root, "", &headers::headers_of(root, "").unwrap()).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

#[test]
fn a_folder_whose_headers_cannot_be_read_is_an_error_not_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let users = collection::create_folder(root, "", "Users").unwrap();
    std::fs::write(root.join("users/_folder.toml"), "headers = 3\n").unwrap();

    let error = headers::headers_chain(root, &format!("{users}/a.req.toml")).unwrap_err();
    assert_eq!(
        error.to_string(),
        "`headers` must be a list of [[headers]] tables"
    );
}
