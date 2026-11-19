#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::fs;

use wbl_core::example::{Example, Param};
use wbl_core::{Body, BodyKind, Header, Request};
use wbl_format::collection::{self, environments};
use wbl_format::{Environment, EnvironmentFile, RequestFile, SecretName, StoredRequest};

fn env() -> Environment {
    Environment {
        name: "Production".into(),
        protected: true,
        variables: [("baseUrl".to_owned(), "https://api.example.com".to_owned())].into(),
        secrets: vec![SecretName {
            name: "apiKey".into(),
            description: "From 1Password".into(),
        }],
    }
}

#[test]
fn an_environment_file_round_trips_and_keeps_comments() {
    let text = EnvironmentFile::new(&env()).to_string();
    assert!(text.contains("protected = true"), "{text}");
    assert!(text.contains("[secrets]"), "{text}");
    assert!(!text.contains("1Password\"\"\""), "{text}");
    assert_eq!(
        EnvironmentFile::parse(&text).unwrap().read().unwrap(),
        env()
    );

    let commented = text.replace("[variables]", "# shared with the team\n[variables]");
    let file = EnvironmentFile::parse(&commented).unwrap();
    assert_eq!(file.updated(&env()).to_string(), commented);
    let unprotected = Environment {
        protected: false,
        ..env()
    };
    let edited = file.updated(&unprotected).to_string();
    assert!(edited.contains("# shared with the team"));
    assert_eq!(
        EnvironmentFile::parse(&edited).unwrap().read().unwrap(),
        unprotected
    );
}

#[test]
fn secret_values_cannot_be_written_in_an_environment_file() {
    let text = "format = 1\nname = \"x\"\n[secrets]\napiKey = \"sk-live-123\"\n";
    let error = EnvironmentFile::parse(text).unwrap().read().unwrap_err();
    assert_eq!(
        error.to_string(),
        "`secrets.apiKey` must be a table with a description; values belong in the keychain"
    );
}

fn stored_with_examples() -> StoredRequest {
    StoredRequest {
        name: "Create user".into(),
        order: 10,
        request: Request {
            method: "POST".into(),
            url: "{{baseUrl}}/users".into(),
            headers: vec![],
            body: Body::Text {
                kind: BodyKind::Json,
                text: "{}".into(),
            },
        },
        examples: vec![
            Example {
                variables: [("role".to_owned(), "admin".to_owned())].into(),
                query: vec![Param {
                    name: "notify".into(),
                    value: "true".into(),
                    enabled: true,
                }],
                headers: vec![Header {
                    name: "X-Tenant".into(),
                    value: "{{tenant}}".into(),
                    enabled: false,
                }],
                body: Some(Body::Text {
                    kind: BodyKind::Json,
                    text: "{\n  \"role\": \"{{role}}\"\n}\n".into(),
                }),
                order: 10,
                ..Example::named("b1e0", "Example A")
            },
            Example {
                body: Some(Body::None),
                ..Example::named("c2", "No body")
            },
            Example::named("d3", "Empty"),
        ],
        settings: wbl_core::RequestSettings {
            timeout_ms: Some(5000),
            follow_redirects: false,
            verify_tls: true,
        },
        auth: Default::default(),
        variables: Default::default(),
        captures: vec![],
        assertions: vec![],
    }
}

#[test]
fn examples_round_trip_in_the_request_file() {
    let stored = stored_with_examples();
    let text = RequestFile::new(&stored).to_string();

    assert!(text.contains("[[examples]]"), "{text}");
    assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), stored);
    assert_eq!(
        RequestFile::parse(&text)
            .unwrap()
            .updated(&stored)
            .to_string(),
        text
    );
}

#[test]
fn environments_live_in_their_folder_with_local_overrides_beside() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();

    let id = environments::create(root, &env()).unwrap();
    let staging = environments::create(
        root,
        &Environment {
            name: "Staging".into(),
            protected: false,
            ..env()
        },
    )
    .unwrap();
    environments::save_local(
        root,
        &id,
        &[("baseUrl".to_owned(), "http://localhost:8080".to_owned())].into(),
    )
    .unwrap();
    fs::write(root.join("environments/broken.toml"), "name =").unwrap();

    let listed = environments::list(root).unwrap();
    let read = environments::read(root, &id).unwrap();

    assert_eq!(id, "production");
    assert_eq!(staging, "staging");
    assert_eq!(
        listed.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
        vec!["broken", "production", "staging"]
    );
    assert!(listed[0].error.is_some());
    assert_eq!(read.environment, env());
    assert_eq!(
        read.local.get("baseUrl").map(String::as_str),
        Some("http://localhost:8080")
    );
    assert!(
        fs::read_to_string(root.join(".gitignore"))
            .unwrap()
            .contains(".local/")
    );
    assert!(root.join(".local/production.toml").is_file());
}

#[test]
fn saving_an_environment_detects_changes_meanwhile_and_names_are_unique() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let id = environments::create(root, &env()).unwrap();
    let second = environments::create(root, &env()).unwrap();
    let read = environments::read(root, &id).unwrap();
    let edited = Environment {
        protected: false,
        ..read.environment.clone()
    };

    environments::save(root, &id, &edited, &read.hash).unwrap();
    let stale = environments::save(root, &id, &edited, &read.hash).unwrap_err();
    environments::delete(root, &second).unwrap();

    assert_eq!(second, "production-2");
    assert_eq!(stale, wbl_format::FormatError::Conflict);
    assert_eq!(environments::list(root).unwrap().len(), 1);
    assert!(environments::read(root, "../collection").is_err());
}

#[test]
fn collection_variables_are_read_and_written_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    collection::create(root, "API").unwrap();
    let vars: BTreeMap<String, String> = [("apiVersion".to_owned(), "v2".to_owned())].into();

    collection::save_variables(root, &vars).unwrap();

    assert_eq!(collection::variables(root).unwrap(), vars);
    assert_eq!(collection::load(root).unwrap().name, "API");
}
