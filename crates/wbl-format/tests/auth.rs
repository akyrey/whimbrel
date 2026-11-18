#![allow(clippy::unwrap_used, clippy::expect_used)]

use wbl_core::auth::{Auth, KeyLocation};
use wbl_format::CollectionTls;
use wbl_format::collection::{self, auth};

fn bearer(token: &str) -> Auth {
    Auth::Bearer { token: token.into() }
}

#[test]
fn folders_and_the_collection_hold_auth_that_requests_inherit() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let users = collection::create_folder(root, "", "Users").unwrap();
    let admin = collection::create_folder(root, &users, "Admin").unwrap();

    auth::save_auth(root, "", &bearer("{{collectionToken}}")).unwrap();
    auth::save_auth(root, &users, &Auth::ApiKey { name: "X-Key".into(), value: "{{key}}".into(), location: KeyLocation::Header }).unwrap();

    let chain = auth::auth_chain(root, &format!("{admin}/list.req.toml")).unwrap();
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0], Auth::Inherit, "Admin sets none");
    assert!(matches!(chain[1], Auth::ApiKey { .. }));
    assert_eq!(chain[2], bearer("{{collectionToken}}"));
    assert_eq!(auth::auth_chain(root, "ping.req.toml").unwrap(), vec![bearer("{{collectionToken}}")]);
    assert_eq!(collection::load(root).unwrap().name, "API", "collection.toml still reads");
}

#[test]
fn inherit_removes_the_table_and_a_folder_keeps_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let users = collection::create_folder(root, "", "Users").unwrap();

    auth::save_auth(root, &users, &Auth::None).unwrap();
    assert_eq!(auth::auth_of(root, &users).unwrap(), Auth::None);
    auth::save_auth(root, &users, &Auth::Inherit).unwrap();

    let text = std::fs::read_to_string(root.join("users/_folder.toml")).unwrap();
    assert!(!text.contains("[auth]"), "{text}");
    assert!(text.contains("name = \"Users\""));
    assert!(auth::save_auth(root, "../x", &Auth::None).is_err());
    assert!(auth::save_auth(root, "nowhere", &Auth::None).is_err());
}

#[test]
fn the_collection_keeps_its_tls_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let settings = CollectionTls { ca_file: "certs/ca.pem".into(), client_cert_file: String::new() };

    auth::save_tls(root, &settings).unwrap();

    assert_eq!(auth::tls(root).unwrap(), settings);
    assert_eq!(auth::tls_file(root, "certs/ca.pem"), root.join("certs/ca.pem"));
    assert_eq!(auth::tls_file(root, "/etc/ca.pem"), std::path::PathBuf::from("/etc/ca.pem"));
    auth::save_tls(root, &CollectionTls::default()).unwrap();
    assert!(!std::fs::read_to_string(root.join("collection.toml")).unwrap().contains("[tls]"));
}

#[test]
fn a_bad_auth_type_is_named() {
    let text = "format = 1\nname = \"x\"\nmethod = \"GET\"\nurl = \"http://x\"\n\n[auth]\ntype = \"digest\"\n";
    let error = wbl_format::RequestFile::parse(text).unwrap().read().unwrap_err();
    assert_eq!(error.to_string(), "`auth.type` must be one of none, basic, bearer, api_key, oauth2");
}
