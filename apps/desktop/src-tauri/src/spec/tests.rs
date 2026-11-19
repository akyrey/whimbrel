#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use wbl_http::SendOptions;

use super::*;
use crate::ipc::CollectionCreateArgs;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../crates/wbl-openapi/tests/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn sender() -> Sender {
    Sender::new(SendOptions::default()).unwrap()
}

/// Serves `files` by path; anything else is a 404.
async fn server(files: HashMap<&'static str, String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let files = Arc::new(files);
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let files = Arc::clone(&files);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).into_owned();
                let path = head.split_whitespace().nth(1).unwrap_or("/").to_owned();
                let answer = match files.get(path.as_str()) {
                    Some(body) => format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/yaml\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    ),
                    None => {
                        "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                            .into()
                    }
                };
                let _ = socket.write_all(answer.as_bytes()).await;
            });
        }
    });
    base
}

const SPEC: &str = "openapi: 3.1.0\ninfo: {title: Remote, version: '1'}\nservers: [{url: /v1}]\npaths:\n  /pets:\n    get:\n      operationId: listPets\n      responses:\n        '200': {description: ok, content: {application/json: {schema: {$ref: 'schemas.yaml#/Pet'}}}}\n";
const SCHEMAS: &str = "Pet: {type: object, properties: {name: {type: string}}}\n";

#[tokio::test]
async fn a_file_is_summarised_before_import() {
    let read = read(
        &sender(),
        &SpecSource::File {
            path: fixture("openapi30.yaml"),
        },
        false,
    )
    .await
    .unwrap();

    let summary = summary(&read.api, read.remote);

    assert_eq!(summary.title, "Users");
    assert_eq!(summary.version_text, "3.0.3");
    assert_eq!(summary.operations, 3);
    assert_eq!(summary.folders, ["users"]);
    assert_eq!(summary.environments, ["Production", "Local"]);
    assert!(summary.remote.is_empty());
}

#[tokio::test]
async fn a_url_is_fetched_and_remote_references_wait_for_consent() {
    let base = server(HashMap::from([
        ("/api/spec.yaml", SPEC.to_owned()),
        ("/api/schemas.yaml", SCHEMAS.to_owned()),
    ]))
    .await;
    let source = SpecSource::Url {
        url: format!("{base}/api/spec.yaml"),
    };

    let without = read(&sender(), &source, false).await.unwrap();
    let with = read(&sender(), &source, true).await.unwrap();

    assert_eq!(without.remote, [format!("{base}/api/schemas.yaml")]);
    assert!(with.remote.is_empty());
    assert_eq!(
        with.api.schemas["Pet"]["properties"]["name"]["type"],
        "string"
    );
    assert_eq!(
        with.api.servers[0].url,
        format!("{base}/v1"),
        "a relative server is the spec's own host"
    );
    let missing = read(
        &sender(),
        &SpecSource::Url {
            url: format!("{base}/nope.yaml"),
        },
        false,
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code, "openapi_load");
    let wrong = read(
        &sender(),
        &SpecSource::Url {
            url: "ftp://x".into(),
        },
        false,
    )
    .await
    .unwrap_err();
    assert_eq!(wrong.code, "invalid_url");
    let not_api = read(
        &sender(),
        &SpecSource::Url {
            url: format!("{base}/api/schemas.yaml"),
        },
        false,
    )
    .await
    .unwrap_err();
    assert_eq!(not_api.code, "openapi_unknown");
}

#[tokio::test]
async fn imports_into_a_new_collection_or_a_folder_of_an_open_one() {
    let sender = sender();
    let collections = Collections::default();
    let fresh = tempfile::tempdir().unwrap();
    let source = SpecSource::File {
        path: fixture("openapi30.yaml"),
    };
    let read = read(&sender, &source, false).await.unwrap();

    let new = install(
        &collections,
        &read.api,
        &read.source,
        &ImportTarget::New {
            folder: fresh.path().to_string_lossy().into_owned(),
            name: "Users".into(),
        },
    )
    .unwrap();
    assert_eq!(new.folder, "");
    assert_eq!(
        new.installed.requests["listUsers"],
        "users/listusers.req.toml"
    );

    let existing = tempfile::tempdir().unwrap();
    let root = existing
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    collections
        .create(&CollectionCreateArgs {
            root: root.clone(),
            name: "Mine".into(),
        })
        .unwrap();
    let into = install(
        &collections,
        &read.api,
        &read.source,
        &ImportTarget::Existing {
            root: root.clone(),
            folder_name: "Users API".into(),
        },
    )
    .unwrap();
    assert_eq!(into.folder, "users-api");
    assert_eq!(
        into.installed.requests["getUser"],
        "users-api/users/getuser.req.toml"
    );

    let specs = wbl_openapi::specs(Path::new(&root)).unwrap();
    assert_eq!(specs[0].folder, "users-api");
    let api = open(
        &collections,
        &sender,
        &SpecOpenArgs {
            root: root.clone(),
            dir: specs[0].dir.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(api.operations.len(), 3);
    let checked = validate(
        &collections,
        &sender,
        &SpecValidateArgs {
            root: root.clone(),
            dir: specs[0].dir.clone(),
            operation_id: "getUser".into(),
            status: 200,
            content_type: Some("application/json".into()),
            body: r#"{"id": "x"}"#.into(),
        },
    )
    .await
    .unwrap();
    assert!(checked.checked && !checked.errors.is_empty());
    let gone = open(
        &collections,
        &sender,
        &SpecOpenArgs {
            root,
            dir: "openapi/none".into(),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(gone.code, "not_found");
}

#[tokio::test]
async fn importing_into_a_closed_collection_is_refused() {
    let read = read(
        &sender(),
        &SpecSource::File {
            path: fixture("openapi31.yaml"),
        },
        false,
    )
    .await
    .unwrap();
    let dir = tempfile::tempdir().unwrap();

    let refused = install(
        &Collections::default(),
        &read.api,
        &read.source,
        &ImportTarget::Existing {
            root: dir.path().to_string_lossy().into_owned(),
            folder_name: "x".into(),
        },
    )
    .unwrap_err();

    assert_eq!(refused.code, "not_open");
}

#[tokio::test]
async fn a_kept_spec_is_reimported_from_a_new_file() {
    let sender = sender();
    let collections = Collections::default();
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    collections
        .create(&CollectionCreateArgs {
            root: root.clone(),
            name: "Mine".into(),
        })
        .unwrap();
    let first = read(
        &sender,
        &SpecSource::File {
            path: fixture("openapi30.yaml"),
        },
        false,
    )
    .await
    .unwrap();
    let done = install(
        &collections,
        &first.api,
        &first.source,
        &ImportTarget::Existing {
            root: root.clone(),
            folder_name: "Users".into(),
        },
    )
    .unwrap();
    let changed = dir.path().join("v2.yaml");
    let text = std::fs::read_to_string(fixture("openapi30.yaml"))
        .unwrap()
        .replace("operationId: getUser", "operationId: fetchUser");
    std::fs::write(&changed, text).unwrap();
    let args = SpecReimportArgs {
        root: root.clone(),
        dir: done.installed.spec_dir.clone(),
        source: SpecSource::File {
            path: changed.to_string_lossy().into_owned(),
        },
        allow_remote: false,
        choices: None,
    };

    let planned = reimport_plan(&collections, &sender, &args).await.unwrap();
    let applied = reimport_apply(&collections, &sender, &args).await.unwrap();

    assert_eq!(planned.changes.len(), 2, "{:?}", planned.changes);
    assert_eq!(applied, planned);
    let kept = wbl_openapi::specs(Path::new(&root)).unwrap();
    assert!(kept[0].operations.contains_key("fetchUser"));
}
