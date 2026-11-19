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
        examples: vec![],
        settings: Default::default(),
        auth: Default::default(),
        variables: Default::default(),
        captures: vec![],
        assertions: vec![],
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
        "`body.kind` must be one of json, xml, text, form, multipart, file"
    );
}

#[test]
fn a_file_that_is_not_toml_says_where() {
    let error = RequestFile::parse("name = \nmethod = \"GET\"").unwrap_err();
    assert!(matches!(error, FormatError::Syntax(_)));
    assert!(error.to_string().contains("line 1"), "{error}");
}

#[test]
fn request_variables_round_trip() {
    let stored = StoredRequest {
        variables: [("userId".to_owned(), "42".to_owned())].into(),
        ..stored()
    };

    let text = RequestFile::new(&stored).to_string();

    assert!(text.contains("[variables]\nuserId = \"42\""), "{text}");
    assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), stored);
    let cleared = StoredRequest {
        variables: Default::default(),
        captures: vec![],
        assertions: vec![],
        ..stored
    };
    assert!(
        !RequestFile::parse(&text)
            .unwrap()
            .updated(&cleared)
            .to_string()
            .contains("[variables]")
    );
}

#[test]
fn captures_and_assertions_are_written_as_documented() {
    use wbl_core::check::{Assertion, Capture, CaptureFrom};
    let stored = StoredRequest {
        captures: vec![Capture {
            var: "userId".into(),
            from: CaptureFrom::Body,
            path: "$.id".into(),
            regex: None,
            secret: false,
        }],
        assertions: vec![
            Assertion::Status { equals: 201 },
            Assertion::Json {
                path: "$.tags".into(),
                equals: Some(serde_json::json!(["a", 1])),
            },
            Assertion::Time { below_ms: 500 },
        ],
        ..stored()
    };

    let text = RequestFile::new(&stored).to_string();

    assert!(
        text.contains("[[captures]]\nvar = \"userId\"\nfrom = \"body\"\npath = \"$.id\"\n"),
        "{text}"
    );
    assert!(
        text.contains("[[assertions]]\ncheck = \"status\"\nequals = 201\n"),
        "{text}"
    );
    assert!(text.contains("equals = [\"a\", 1]"), "{text}");
    assert!(text.contains("below_ms = 500"), "{text}");
    assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), stored);
    let bad = format!("{text}\n[[assertions]]\ncheck = \"nope\"\n");
    assert!(RequestFile::parse(&bad).unwrap().read().is_err());
}

const BASE: &str = "name = \"R\"\nmethod = \"GET\"\nurl = \"http://x\"\n";

#[test]
fn checks_read_every_toml_shape_and_refuse_bad_ones() {
    use wbl_core::check::{Assertion, CaptureFrom};
    let text = format!(
        "{BASE}captures = [{{ var = \"loc\", from = \"header\", path = \"Location\", regex = \"/(\\\\d+)\", secret = true }}]\n\
         [[assertions]]\ncheck = \"json\"\npath = \"$.a\"\nequals = 1.5\n\
         [[assertions]]\ncheck = \"json\"\npath = \"$.b\"\nequals = {{ x = 1979-05-27, y = [true] }}\n\
         [[assertions]]\ncheck = \"header\"\nname = \"ETag\"\n"
    );
    let read = RequestFile::parse(&text).unwrap().read().unwrap();
    assert_eq!(read.captures[0].from, CaptureFrom::Header);
    assert!(read.captures[0].secret);
    assert_eq!(
        read.assertions[0],
        Assertion::Json {
            path: "$.a".into(),
            equals: Some(serde_json::json!(1.5))
        }
    );
    assert_eq!(
        read.assertions[1],
        Assertion::Json {
            path: "$.b".into(),
            equals: Some(serde_json::json!({"x": "1979-05-27", "y": [true]}))
        }
    );
    assert_eq!(
        read.assertions[2],
        Assertion::Header {
            name: "ETag".into(),
            equals: None
        }
    );

    let written = RequestFile::new(&read).to_string();
    assert!(written.contains("secret = true"), "{written}");
    assert_eq!(RequestFile::parse(&written).unwrap().read().unwrap(), read);
    let null = StoredRequest {
        assertions: vec![Assertion::Json {
            path: "$".into(),
            equals: Some(serde_json::Value::Null),
        }],
        ..read
    };
    assert!(
        RequestFile::new(&null)
            .to_string()
            .contains("equals = \"null\"")
    );

    for bad in [
        "[[captures]]\nvar = \"v\"\nfrom = \"cookie\"\n",
        "[[captures]]\nfrom = \"body\"\n",
        "[[assertions]]\ncheck = \"status\"\nequals = \"ok\"\n",
        "[[assertions]]\ncheck = \"status\"\nequals = 99999\n",
        "[[assertions]]\ncheck = \"time\"\nbelow_ms = -1\n",
        "[[assertions]]\nequals = 1\n",
        "captures = 3\n",
    ] {
        let text = format!("{BASE}{bad}");
        assert!(RequestFile::parse(&text).unwrap().read().is_err(), "{bad}");
    }
}

#[test]
fn form_multipart_and_file_bodies_are_written_as_documented() {
    use wbl_core::FormField;
    let upload = StoredRequest {
        request: Request {
            body: Body::Multipart {
                fields: vec![
                    FormField {
                        name: "title".into(),
                        value: "{{title}}".into(),
                        enabled: true,
                        file: false,
                    },
                    FormField {
                        name: "doc".into(),
                        value: "files/a.pdf".into(),
                        enabled: false,
                        file: true,
                    },
                ],
            },
            ..stored().request
        },
        ..stored()
    };

    let text = RequestFile::new(&upload).to_string();

    assert!(text.contains("[body]\nkind = \"multipart\""), "{text}");
    assert!(
        text.contains(
            "[[body.fields]]\nname = \"doc\"\nvalue = \"files/a.pdf\"\nenabled = false\nfile = true"
        ),
        "{text}"
    );
    assert_eq!(RequestFile::parse(&text).unwrap().read().unwrap(), upload);
    let as_file = StoredRequest {
        request: Request {
            body: Body::File {
                path: "big.bin".into(),
            },
            ..upload.request.clone()
        },
        ..upload.clone()
    };
    let changed = RequestFile::parse(&text)
        .unwrap()
        .updated(&as_file)
        .to_string();
    assert!(
        changed.contains("kind = \"file\"\npath = \"big.bin\""),
        "{changed}"
    );
    assert!(!changed.contains("fields"), "{changed}");
    let bad = format!("{BASE}[body]\nkind = \"zip\"\n");
    assert!(
        RequestFile::parse(&bad)
            .unwrap()
            .read()
            .unwrap_err()
            .to_string()
            .contains("multipart")
    );
}
