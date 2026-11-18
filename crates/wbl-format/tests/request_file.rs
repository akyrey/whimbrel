#![allow(clippy::unwrap_used, clippy::expect_used)]

use wbl_core::{Body, BodyKind, Header, Request};
use wbl_format::{FormatError, RequestFile, StoredRequest};

fn stored() -> StoredRequest {
    StoredRequest {
        name: "Create user".into(),
        order: 20,
        request: Request {
            method: "POST".into(),
            url: "{{baseUrl}}/users".into(),
            headers: vec![
                Header {
                    name: "Content-Type".into(),
                    value: "application/json".into(),
                    enabled: true,
                },
                Header {
                    name: "X-Debug".into(),
                    value: "1".into(),
                    enabled: false,
                },
            ],
            body: Body::Text {
                kind: BodyKind::Json,
                text: "{\n  \"name\": \"ada\"\n}\n".into(),
            },
        },
    }
}

const CANONICAL: &str = r#"format = 1
name = "Create user"
order = 20
method = "POST"
url = "{{baseUrl}}/users"

[[headers]]
name = "Content-Type"
value = "application/json"

[[headers]]
name = "X-Debug"
value = "1"
enabled = false

[body]
kind = "json"
text = """
{
  "name": "ada"
}
"""
"#;

#[test]
fn a_new_file_has_the_canonical_layout() {
    assert_eq!(RequestFile::new(&stored()).to_string(), CANONICAL);
}

#[test]
fn reads_what_it_writes() {
    let file = RequestFile::parse(CANONICAL).unwrap();
    assert_eq!(file.read().unwrap(), stored());
}

const HAND_EDITED: &str = r#"# Creates a user; see the API docs.
format = 1
name = "Create user"   # shown in the tree
order = 20
method = "POST"
url = "{{baseUrl}}/users"
owner = "team-a"       # not ours: must survive

[[headers]]
name = "Content-Type"
value = "application/json"

[[headers]]
name = "X-Debug"
value = "1"
enabled = false

[body]
kind = "json"
text = """
{
  "name": "ada"
}
"""
"#;

#[test]
fn saving_an_unchanged_request_changes_nothing() {
    let file = RequestFile::parse(HAND_EDITED).unwrap();
    let request = file.read().unwrap();

    assert_eq!(file.updated(&request).to_string(), HAND_EDITED);
}

#[test]
fn editing_one_field_changes_only_its_value_and_keeps_comments_and_unknown_keys() {
    let file = RequestFile::parse(HAND_EDITED).unwrap();
    let renamed = StoredRequest {
        name: "Create a user".into(),
        ..file.read().unwrap()
    };

    let text = file.updated(&renamed).to_string();

    assert_eq!(
        text,
        HAND_EDITED.replace(
            "name = \"Create user\"   # shown in the tree",
            "name = \"Create a user\"   # shown in the tree"
        )
    );
}

#[test]
fn removing_the_body_and_a_header() {
    let file = RequestFile::parse(HAND_EDITED).unwrap();
    let mut changed = file.read().unwrap();
    changed.request.body = Body::None;
    changed.request.headers.truncate(1);

    let text = file.updated(&changed).to_string();

    assert!(!text.contains("[body]"), "{text}");
    assert!(!text.contains("X-Debug"), "{text}");
    assert!(text.contains("owner = \"team-a\""));
    assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), changed);
}

#[test]
fn adding_a_body_to_a_request_without_one() {
    let mut get = stored();
    get.request.body = Body::None;
    let file = RequestFile::new(&get);

    let text = file.updated(&stored()).to_string();

    assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), stored());
}

#[test]
fn bodies_with_quotes_and_backslashes_survive() {
    let mut tricky = stored();
    tricky.request.body = Body::Text {
        kind: BodyKind::Text,
        text: "a \"\"\" b\\n c '''\r\n\ttab".into(),
    };

    let text = RequestFile::new(&tricky).to_string();

    assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), tricky);
}

#[test]
fn a_newer_format_is_refused() {
    let error = RequestFile::parse(&CANONICAL.replace("format = 1", "format = 2"))
        .unwrap()
        .read()
        .unwrap_err();
    assert_eq!(
        error,
        FormatError::TooNew {
            found: 2,
            supported: 1
        }
    );
}

#[test]
fn missing_or_wrong_fields_are_named() {
    let no_method = CANONICAL.replace("method = \"POST\"\n", "");
    let bad_order = CANONICAL.replace("order = 20", "order = \"first\"");
    let bad_kind = CANONICAL.replace("kind = \"json\"", "kind = \"yaml\"");

    let error = |text: &str| {
        RequestFile::parse(text)
            .unwrap()
            .read()
            .unwrap_err()
            .to_string()
    };

    assert_eq!(error(&no_method), "`method` is missing");
    assert_eq!(error(&bad_order), "`order` must be a whole number");
    assert_eq!(
        error(&bad_kind),
        "`body.kind` must be one of json, xml, text"
    );
}

#[test]
fn a_file_that_is_not_toml_says_where() {
    let error = RequestFile::parse("name = \nmethod = \"GET\"").unwrap_err();
    assert!(matches!(error, FormatError::Syntax(_)));
    assert!(error.to_string().contains("line 1"), "{error}");
}
