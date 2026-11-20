#![allow(clippy::unwrap_used, clippy::expect_used)]

use wbl_core::{Body, Request};
use wbl_format::StoredRequest;

use super::*;

fn stored(name: &str) -> StoredRequest {
    StoredRequest {
        name: name.into(),
        order: 0,
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
        captures: vec![],
        assertions: vec![],
    }
}

fn secrets() -> Secrets {
    Secrets::new(std::sync::Arc::new(
        roost_keychain::MemorySecretStore::default(),
    ))
}

fn node(root: &str, path: &str) -> NodeArgs {
    NodeArgs {
        root: root.into(),
        path: path.into(),
    }
}

fn created() -> (tempfile::TempDir, Collections, String) {
    let dir = tempfile::tempdir().unwrap();
    let state = Collections::default();
    let root = dir.path().to_string_lossy().into_owned();
    state
        .create(&CollectionCreateArgs {
            root: root.clone(),
            name: "API".into(),
        })
        .unwrap();
    (dir, state, root)
}

#[test]
fn only_open_collections_can_be_touched() {
    let dir = tempfile::tempdir().unwrap();
    collection::create(dir.path(), "Elsewhere").unwrap();
    let root = dir.path().to_string_lossy().into_owned();
    let state = Collections::default();

    let refused = tree(&state, &CollectionArgs { root: root.clone() }).unwrap_err();
    state.open(&root).unwrap();
    let tree_after = tree(&state, &CollectionArgs { root: root.clone() }).unwrap();
    state.close(&root).unwrap();

    assert_eq!(refused.code, "not_open");
    assert_eq!(tree_after.collection.name, "Elsewhere");
    assert_eq!(
        read(&state, &node(&root, "x.req.toml")).unwrap_err().code,
        "not_open"
    );
}

#[test]
fn opening_a_folder_that_is_not_a_collection_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let error = Collections::default()
        .open(&dir.path().to_string_lossy())
        .unwrap_err();
    assert_eq!(error.code, "invalid_file");
    assert_eq!(
        Collections::default()
            .open("/no/such/folder")
            .unwrap_err()
            .code,
        "not_found"
    );
}

#[test]
fn create_read_save_and_conflict() {
    let (_dir, state, root) = created();
    let path = create_request(
        &state,
        &secrets(),
        &RequestCreateArgs {
            root: root.clone(),
            folder: String::new(),
            stored: stored("Ping"),
        },
    )
    .unwrap();
    let read_back = read(&state, &node(&root, &path)).unwrap();
    let edited = StoredRequest {
        name: "Ping 2".into(),
        ..read_back.stored.clone()
    };
    let save_args = RequestSaveArgs {
        root: root.clone(),
        path: path.clone(),
        stored: edited,
        hash: read_back.hash.clone(),
    };

    let new_hash = save(&state, &secrets(), &save_args).unwrap();
    let stale = save(&state, &secrets(), &save_args).unwrap_err();

    assert_ne!(new_hash, read_back.hash);
    assert_eq!(stale.code, "conflict");
}

#[test]
fn folders_rename_duplicate_move_order_and_delete() {
    let (_dir, state, root) = created();
    let path = create_request(
        &state,
        &secrets(),
        &RequestCreateArgs {
            root: root.clone(),
            folder: String::new(),
            stored: stored("Ping"),
        },
    )
    .unwrap();
    let folder = create_folder(
        &state,
        &NodeNameArgs {
            root: root.clone(),
            path: String::new(),
            name: "Health".into(),
        },
    )
    .unwrap();

    let renamed = rename(
        &state,
        &NodeNameArgs {
            root: root.clone(),
            path: path.clone(),
            name: "Pong".into(),
        },
    )
    .unwrap();
    let copy = duplicate(&state, &node(&root, &renamed)).unwrap();
    let moved = move_to(
        &state,
        &NodeMoveArgs {
            root: root.clone(),
            path: copy,
            folder: folder.clone(),
        },
    )
    .unwrap();
    order(
        &state,
        &NodeOrderArgs {
            root: root.clone(),
            path: folder.clone(),
            order: 1,
        },
    )
    .unwrap();
    delete(&state, &node(&root, &renamed)).unwrap();

    let view = tree(&state, &CollectionArgs { root: root.clone() }).unwrap();
    assert_eq!(moved, "health/pong-copy.req.toml");
    assert_eq!(view.collection.children.len(), 1, "only the folder is left");
    assert_eq!(
        delete(&state, &node(&root, "../x")).unwrap_err().code,
        "invalid_path"
    );
}

#[test]
fn folder_and_collection_auth_and_tls_are_read_and_saved() {
    let (_dir, state, root) = created();
    let auth = wbl_core::auth::Auth::Bearer {
        token: "{{token}}".into(),
    };

    auth_save(
        &state,
        &secrets(),
        &crate::ipc::AuthSaveArgs {
            root: root.clone(),
            path: String::new(),
            auth: auth.clone(),
        },
    )
    .unwrap();
    tls_save(
        &state,
        &crate::ipc::TlsSaveArgs {
            root: root.clone(),
            tls: wbl_format::CollectionTls {
                ca_file: "ca.pem".into(),
                client_cert_file: String::new(),
            },
        },
    )
    .unwrap();

    assert_eq!(auth_read(&state, &node(&root, "")).unwrap(), auth);
    assert_eq!(
        tls_read(&state, &CollectionArgs { root: root.clone() })
            .unwrap()
            .ca_file,
        "ca.pem"
    );
}

#[test]
fn folder_and_collection_headers_are_read_and_saved() {
    let (_dir, state, root) = created();
    let headers = vec![wbl_core::Header {
        name: "X-Tenant".into(),
        value: "{{tenant}}".into(),
        enabled: true,
    }];

    headers_save(
        &state,
        &secrets(),
        &crate::ipc::HeadersSaveArgs {
            root: root.clone(),
            path: String::new(),
            headers: headers.clone(),
        },
    )
    .unwrap();

    assert_eq!(headers_read(&state, &node(&root, "")).unwrap(), headers);
    assert_eq!(
        headers_read(&state, &node(&root, "../x")).unwrap_err().code,
        "invalid_path"
    );
}
