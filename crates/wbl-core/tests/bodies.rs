#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 8.1: form, multipart and file bodies through interpolation and cURL.

use std::collections::BTreeMap;

use wbl_core::curl::{parse, to_curl};
use wbl_core::interpolate::{Scope, SystemDynamic, Value, names_used, resolve};
use wbl_core::{Body, FormField, Header, Request};

fn field(name: &str, value: &str, file: bool) -> FormField {
    FormField {
        name: name.into(),
        value: value.into(),
        enabled: true,
        file,
    }
}

fn request(body: Body) -> Request {
    Request {
        method: "POST".into(),
        url: "https://x.test/up".into(),
        headers: vec![],
        body,
    }
}

#[test]
fn variables_in_fields_and_paths_resolve_and_disabled_fields_stay() {
    let scope = Scope::new(BTreeMap::from([
        ("who".to_owned(), Value::Plain("ada".into())),
        ("dir".to_owned(), Value::Plain("/tmp".into())),
    ]));
    let off = FormField {
        enabled: false,
        ..field("x", "{{nope}}", false)
    };
    let form = request(Body::Multipart {
        fields: vec![
            field("name", "{{who}}", false),
            field("doc", "{{dir}}/a.pdf", true),
            off.clone(),
        ],
    });

    let done = resolve(&form, std::slice::from_ref(&scope), &SystemDynamic).unwrap();

    assert_eq!(
        done.request.body,
        Body::Multipart {
            fields: vec![
                field("name", "ada", false),
                field("doc", "/tmp/a.pdf", true),
                off
            ]
        }
    );
    assert_eq!(names_used(&form), ["who", "dir"]);
    let file = resolve(
        &request(Body::File {
            path: "{{dir}}/b.bin".into(),
        }),
        &[scope],
        &SystemDynamic,
    )
    .unwrap();
    assert_eq!(
        file.request.body,
        Body::File {
            path: "/tmp/b.bin".into()
        }
    );
}

#[test]
fn content_types_follow_the_kind_unless_a_header_says() {
    assert_eq!(
        request(Body::Form { fields: vec![] }).default_content_type(),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(
        request(Body::File { path: "a".into() }).default_content_type(),
        Some("application/octet-stream")
    );
    assert_eq!(
        request(Body::Multipart { fields: vec![] }).default_content_type(),
        None
    );
    let mut own = request(Body::File { path: "a".into() });
    own.headers.push(Header {
        name: "content-type".into(),
        value: "image/png".into(),
        enabled: true,
    });
    assert_eq!(own.default_content_type(), None);
}

#[test]
fn curl_round_trips_forms_and_files() {
    let bodies = [
        Body::Form {
            fields: vec![field("a", "1 2", false), field("b", "x=y", false)],
        },
        Body::Multipart {
            fields: vec![
                field("name", "ada", false),
                field("doc", "/tmp/it's.pdf", true),
            ],
        },
        Body::File {
            path: "/tmp/data.bin".into(),
        },
    ];
    for body in bodies {
        let original = request(body);
        let command = to_curl(&original);
        let back = parse(&command).unwrap();
        assert_eq!(back.request.body, original.body, "{command}");
        assert_eq!(back.request.method, "POST");
    }
    let typed = parse("curl https://x.test -F 'doc=@/a.png;type=image/png'").unwrap();
    assert_eq!(
        typed.request.body,
        Body::Multipart {
            fields: vec![field("doc", "/a.png", true)]
        }
    );
    let bare = parse("curl https://x.test --data-urlencode 'just text'").unwrap();
    assert_eq!(
        bare.request.body,
        Body::Form {
            fields: vec![field("", "just text", false)]
        }
    );
}

#[test]
fn secrets_are_masked_in_every_body_kind() {
    use wbl_core::interpolate::{MASK, mask_request};
    let secrets = vec!["s3cret".to_owned()];
    let form = request(Body::Form {
        fields: vec![field("token", "s3cret", false)],
    });
    let file = request(Body::File {
        path: "/keys/s3cret.pem".into(),
    });

    let Body::Form { fields } = mask_request(&form, &secrets).body else {
        panic!()
    };
    assert_eq!(fields[0].value, MASK);
    assert_eq!(
        mask_request(&file, &secrets).body,
        Body::File {
            path: format!("/keys/{MASK}.pem")
        }
    );
}

#[test]
fn relative_file_paths_are_taken_from_the_collection() {
    let base = std::path::Path::new("/work/api");
    let multipart = Body::Multipart {
        fields: vec![
            field("doc", "files/a.pdf", true),
            field("abs", "/etc/b", true),
            field("t", "files/c", false),
        ],
    };

    let Body::Multipart { fields } = multipart.with_files_under(base) else {
        panic!()
    };

    assert_eq!(fields[0].value, "/work/api/files/a.pdf");
    assert_eq!(fields[1].value, "/etc/b");
    assert_eq!(fields[2].value, "files/c", "text fields are not paths");
    assert_eq!(
        Body::File {
            path: "x.bin".into()
        }
        .with_files_under(base),
        Body::File {
            path: "/work/api/x.bin".into()
        }
    );
    assert_eq!(
        Body::File {
            path: String::new()
        }
        .with_files_under(base),
        Body::File {
            path: String::new()
        }
    );
    assert_eq!(Body::None.with_files_under(base), Body::None);
}
