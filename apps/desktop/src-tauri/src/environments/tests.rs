#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing_subscriber::fmt::MakeWriter;
use wbl_core::example::Example;
use wbl_core::{Body, Header, Request};
use wbl_format::{Environment, SecretName, StoredRequest};
use wbl_http::SendOptions;

use super::*;
use crate::collections::{create_request, save as save_request};
use crate::commands::{preview, send};
use crate::ipc::{CollectionCreateArgs, RequestCreateArgs, RequestSaveArgs, RequestSendArgs};
use crate::state::AppState;

const SENTINEL: &str = "sentinel-7f3a9c";

struct Fixture {
    _dir: tempfile::TempDir,
    root: String,
    collections: Collections,
    secrets: Secrets,
}

fn fixture() -> Fixture {
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
            name: "API".into(),
        })
        .unwrap();
    let secrets = Secrets::new(Arc::new(roost_keychain::MemorySecretStore::default()));
    Fixture {
        _dir: dir,
        root,
        collections,
        secrets,
    }
}

fn production() -> Environment {
    Environment {
        name: "Production".into(),
        protected: true,
        variables: [("base".to_owned(), "http://placeholder".to_owned())].into(),
        secrets: vec![SecretName {
            name: "apiKey".into(),
            description: String::new(),
        }],
    }
}

fn request(url: &str) -> Request {
    Request {
        method: "GET".into(),
        url: url.into(),
        headers: vec![Header {
            name: "X-Api-Key".into(),
            value: "{{apiKey}}".into(),
            enabled: true,
        }],
        body: Body::None,
    }
}

fn send_args(
    f: &Fixture,
    env: &str,
    request: Request,
    example: Option<Example>,
) -> RequestSendArgs {
    RequestSendArgs {
        request_id: "tab".into(),
        request,
        root: Some(f.root.clone()),
        environment: Some(env.into()),
        example,
        settings: None,
        name: None,
        path: None,
        auth: None,
        variables: None,
    }
}

#[test]
fn environments_are_created_read_saved_and_deleted_with_their_secrets() {
    let f = fixture();
    let id = create(
        &f.collections,
        &EnvironmentCreateArgs {
            root: f.root.clone(),
            environment: production(),
        },
    )
    .unwrap();
    let args = EnvironmentArgs {
        root: f.root.clone(),
        id: id.clone(),
    };
    set_secret(
        &f.collections,
        &f.secrets,
        &SecretSetArgs {
            root: f.root.clone(),
            environment: id.clone(),
            name: "apiKey".into(),
            value: SENTINEL.into(),
        },
    )
    .unwrap();

    let view = read(&f.collections, &f.secrets, &args).unwrap();
    let json = serde_json::to_string(&view).unwrap();
    let saved = save(
        &f.collections,
        &EnvironmentSaveArgs {
            root: f.root.clone(),
            id: id.clone(),
            environment: Environment {
                protected: false,
                ..production()
            },
            hash: view.read.hash.clone(),
        },
    )
    .unwrap();
    let listed = list(
        &f.collections,
        &CollectionArgs {
            root: f.root.clone(),
        },
    )
    .unwrap();
    delete(&f.collections, &f.secrets, &args).unwrap();

    assert_eq!(
        view.secrets,
        vec![SecretState {
            name: "apiKey".into(),
            set: true
        }]
    );
    assert!(
        !json.contains(SENTINEL),
        "values never go to the frontend: {json}"
    );
    assert_ne!(saved, view.read.hash);
    assert!(!listed[0].protected);
    assert_eq!(
        f.secrets
            .get(&collection::id(Path::new(&f.root)).unwrap(), &id, "apiKey")
            .unwrap(),
        None
    );
}

#[test]
fn only_declared_secrets_can_be_set_and_clearing_unsets() {
    let f = fixture();
    let id = create(
        &f.collections,
        &EnvironmentCreateArgs {
            root: f.root.clone(),
            environment: production(),
        },
    )
    .unwrap();
    let undeclared = set_secret(
        &f.collections,
        &f.secrets,
        &SecretSetArgs {
            root: f.root.clone(),
            environment: id.clone(),
            name: "other".into(),
            value: "x".into(),
        },
    )
    .unwrap_err();
    set_secret(
        &f.collections,
        &f.secrets,
        &SecretSetArgs {
            root: f.root.clone(),
            environment: id.clone(),
            name: "apiKey".into(),
            value: "v".into(),
        },
    )
    .unwrap();
    clear_secret(
        &f.collections,
        &f.secrets,
        &SecretArgs {
            root: f.root.clone(),
            environment: id.clone(),
            name: "apiKey".into(),
        },
    )
    .unwrap();

    let view = read(
        &f.collections,
        &f.secrets,
        &EnvironmentArgs {
            root: f.root.clone(),
            id,
        },
    )
    .unwrap();
    assert_eq!(undeclared.code, "not_found");
    assert!(!view.secrets[0].set);
}

#[test]
fn variables_resolve_by_precedence_and_missing_ones_are_named() {
    let f = fixture();
    let id = create(
        &f.collections,
        &EnvironmentCreateArgs {
            root: f.root.clone(),
            environment: production(),
        },
    )
    .unwrap();
    save_variables(
        &f.collections,
        &VariablesSaveArgs {
            root: f.root.clone(),
            id: None,
            variables: [
                ("v".to_owned(), "collection".to_owned()),
                ("base".to_owned(), "x".to_owned()),
            ]
            .into(),
        },
    )
    .unwrap();
    save_variables(
        &f.collections,
        &VariablesSaveArgs {
            root: f.root.clone(),
            id: Some(id.clone()),
            variables: [("base".to_owned(), "http://local".to_owned())].into(),
        },
    )
    .unwrap();

    let missing = preview(
        &f.collections,
        &f.secrets,
        &send_args(&f, &id, request("{{base}}/{{v}}/{{nope}}"), None),
    )
    .unwrap_err();
    set_secret(
        &f.collections,
        &f.secrets,
        &SecretSetArgs {
            root: f.root.clone(),
            environment: id.clone(),
            name: "apiKey".into(),
            value: "k".into(),
        },
    )
    .unwrap();
    let example = Example {
        variables: [("v".to_owned(), "example".to_owned())].into(),
        ..Example::named("e", "E")
    };
    let shown = preview(
        &f.collections,
        &f.secrets,
        &send_args(&f, &id, request("{{base}}/{{v}}"), Some(example)),
    )
    .unwrap();

    let own = |example: Option<Example>| RequestSendArgs {
        variables: Some(
            [
                ("v".to_owned(), "request".to_owned()),
                ("base".to_owned(), "http://own".to_owned()),
            ]
            .into(),
        ),
        ..send_args(&f, &id, request("{{base}}/{{v}}"), example)
    };
    let request_level = preview(&f.collections, &f.secrets, &own(None)).unwrap();
    let example_wins = preview(
        &f.collections,
        &f.secrets,
        &own(Some(Example {
            variables: [("v".to_owned(), "example".to_owned())].into(),
            ..Example::named("e", "E")
        })),
    )
    .unwrap();

    assert_eq!(missing.code, "unresolved");
    assert_eq!(
        missing.detail,
        Some(serde_json::json!({ "undefined": ["nope"], "unsetSecrets": ["apiKey"] }))
    );
    assert_eq!(shown.url, "http://local/example");
    assert_eq!(
        request_level.url, "http://own/request",
        "request beats environment"
    );
    assert_eq!(
        example_wins.url, "http://own/example",
        "example beats request"
    );
    assert_eq!(shown.headers[0].value, wbl_core::interpolate::MASK);
    assert_eq!(
        collection_variables(
            &f.collections,
            &CollectionArgs {
                root: f.root.clone()
            }
        )
        .unwrap()
        .len(),
        2
    );
}

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Answers with the request it received, so a leak through the response would show.
async fn echo_server() -> (String, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(String::new()));
    let record = Arc::clone(&seen);
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let read = socket.read(&mut buf).await.unwrap_or(0);
            *record.lock().unwrap() = String::from_utf8_lossy(&buf[..read]).into_owned();
            let _ = socket
                .write_all(b"HTTP/1.1 204 No Content\r\nconnection: close\r\n\r\n")
                .await;
        }
    });
    (base, seen)
}

fn files_containing(dir: &Path, needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files_containing(&path, needle));
        } else if std::fs::read(&path)
            .unwrap()
            .windows(needle.len())
            .any(|w| w == needle.as_bytes())
        {
            found.push(path.to_string_lossy().into_owned());
        }
    }
    found
}

/// Task 3.8: a secret that is set, sent, previewed and pasted into a request never reaches a
/// file of the collection or a log line.
#[tokio::test]
async fn the_sentinel_secret_never_reaches_a_file_or_the_log() {
    let f = fixture();
    let id = create(
        &f.collections,
        &EnvironmentCreateArgs {
            root: f.root.clone(),
            environment: production(),
        },
    )
    .unwrap();
    let (base, seen) = echo_server().await;
    save_variables(
        &f.collections,
        &VariablesSaveArgs {
            root: f.root.clone(),
            id: Some(id.clone()),
            variables: [("base".to_owned(), base)].into(),
        },
    )
    .unwrap();
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_writer(captured.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let history_dir = tempfile::tempdir().unwrap();
    let state = AppState::new(SendOptions::default())
        .unwrap()
        .with_history(wbl_store::History::open(&history_dir.path().join("whimbrel.db")).unwrap());

    logged("secret_set", async {
        set_secret(
            &f.collections,
            &f.secrets,
            &SecretSetArgs {
                root: f.root.clone(),
                environment: id.clone(),
                name: "apiKey".into(),
                value: SENTINEL.into(),
            },
        )
    })
    .await
    .unwrap();
    let args = send_args(&f, &id, request("{{base}}/users"), None);
    let response = logged(
        "request_send",
        send(
            &state,
            &f.collections,
            &f.secrets,
            &mut |_| {},
            args.clone(),
        ),
    )
    .await
    .unwrap();
    let shown = logged("request_preview", async {
        preview(&f.collections, &f.secrets, &args)
    })
    .await
    .unwrap();
    let stored = StoredRequest {
        name: "Leaky".into(),
        order: 0,
        request: Request {
            headers: vec![Header {
                name: "X-Api-Key".into(),
                value: SENTINEL.into(),
                enabled: true,
            }],
            ..request("{{base}}")
        },
        examples: vec![],
        settings: Default::default(),
        auth: Default::default(),
        variables: Default::default(),
    };
    let refused = logged("request_create", async {
        create_request(
            &f.collections,
            &f.secrets,
            &RequestCreateArgs {
                root: f.root.clone(),
                folder: String::new(),
                stored: stored.clone(),
            },
        )
    })
    .await
    .unwrap_err();
    let path = create_request(
        &f.collections,
        &f.secrets,
        &RequestCreateArgs {
            root: f.root.clone(),
            folder: String::new(),
            stored: StoredRequest {
                request: request("{{base}}"),
                ..stored.clone()
            },
        },
    )
    .unwrap();
    let hash = collection::read_request(Path::new(&f.root), &path)
        .unwrap()
        .hash;
    let refused_save = save_request(
        &f.collections,
        &f.secrets,
        &RequestSaveArgs {
            root: f.root.clone(),
            path,
            stored,
            hash,
        },
    )
    .unwrap_err();

    assert_eq!(response.status, 204);
    assert!(
        seen.lock().unwrap().contains(SENTINEL),
        "the secret was sent"
    );
    assert!(!format!("{shown:?}").contains(SENTINEL));
    assert_eq!(refused.code, "contains_secret");
    assert_eq!(refused_save.code, "contains_secret");
    assert!(!refused.message.contains(SENTINEL));
    assert_eq!(
        files_containing(Path::new(&f.root), SENTINEL),
        Vec::<String>::new()
    );
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("request_send"), "{log}");
    assert!(!log.contains(SENTINEL), "{log}");
    let history = state.history.as_ref().unwrap();
    let recorded = history.list("", 10, 0).unwrap();
    let entry = history.get(recorded[0].id).unwrap().unwrap();
    assert_eq!(entry.request.headers[0].value, wbl_core::interpolate::MASK);
    assert_eq!(
        files_containing(history_dir.path(), SENTINEL),
        Vec::<String>::new()
    );
}
