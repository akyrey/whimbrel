#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Task 6.6: the collection each fixture becomes (snapshots), and the rules behind it.

mod support;

use wbl_core::auth::{Auth, Grant, KeyLocation};
use wbl_core::{Body, BodyKind};
use wbl_openapi::{Generated, GeneratedRequest, generate};

fn request<'a>(generated: &'a Generated, id: &str) -> &'a GeneratedRequest {
    generated
        .requests
        .iter()
        .find(|r| r.operation_id == id)
        .unwrap()
}

#[test]
fn snapshots() {
    for name in [
        "swagger2.yaml",
        "openapi30.yaml",
        "openapi31.yaml",
        "openapi32.yaml",
        "split/main.yaml",
    ] {
        let generated = generate(&support::api(name));
        insta::assert_json_snapshot!(name.replace(['/', '.'], "_"), generated);
    }
}

#[test]
fn requests_from_operations() {
    let generated = generate(&support::api("openapi30.yaml"));

    let list = request(&generated, "listUsers");
    assert_eq!(list.folder, ["users"]);
    assert_eq!(
        list.stored.request.url, "{{baseUrl}}/users?role=admin",
        "only required query parameters"
    );
    let all = list
        .stored
        .examples
        .iter()
        .find(|e| e.name == "All parameters")
        .unwrap();
    let optional: Vec<_> = all
        .query
        .iter()
        .map(|p| (p.name.as_str(), p.value.as_str()))
        .collect();
    assert_eq!(optional, [("limit", "20"), ("ids", "0")]);
    let headers: Vec<_> = list
        .stored
        .request
        .headers
        .iter()
        .map(|h| (h.name.as_str(), h.enabled))
        .collect();
    assert_eq!(headers, [("X-Trace", false), ("Cookie", false)]);

    let get = request(&generated, "getUser");
    assert_eq!(get.stored.request.url, "{{baseUrl}}/users/{{userId}}");
    assert_eq!(get.stored.variables["userId"], "42");
    assert_eq!(get.stored.auth, Auth::Inherit);

    let create = request(&generated, "createUser");
    let Body::Text { kind, text } = &create.stored.request.body else {
        panic!()
    };
    assert_eq!(*kind, BodyKind::Json);
    assert!(text.contains("\"name\": \"Ada\""), "{text}");
    let names: Vec<_> = create
        .stored
        .examples
        .iter()
        .map(|e| (e.id.as_str(), e.name.as_str()))
        .collect();
    assert_eq!(names, [("admin", "admin"), ("member", "member")]);
    assert!(
        matches!(&create.stored.examples[1].body, Some(Body::Text { text, .. }) if text.contains("Grace"))
    );
    assert_eq!(create.stored.request.headers[0].value, "application/json");
}

#[test]
fn environments_and_auth() {
    let generated = generate(&support::api("openapi30.yaml"));

    assert_eq!(
        generated.auth,
        Auth::Bearer {
            token: "{{bearerToken}}".into()
        }
    );
    let names: Vec<_> = generated
        .environments
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(names, ["Production", "Local"]);
    let production = &generated.environments[0];
    assert_eq!(
        production.variables["baseUrl"],
        "https://{{region}}.api.example.com/{{basePath}}"
    );
    assert_eq!(production.variables["region"], "eu");
    assert_eq!(production.secrets[0].name, "bearerToken");
}

#[test]
fn swagger_auth_folders_and_forms() {
    let generated = generate(&support::api("swagger2.yaml"));

    let inventory = request(&generated, "getInventory");
    assert_eq!(
        inventory.folder,
        ["store"],
        "untagged: the first path segment"
    );
    assert_eq!(
        inventory.stored.auth,
        Auth::ApiKey {
            name: "api_key".into(),
            value: "{{apiKey}}".into(),
            location: KeyLocation::Header
        }
    );
    let add = request(&generated, "addPet");
    assert_eq!(
        add.stored.auth,
        Auth::None,
        "implicit flow only: no auth Whimbrel can run"
    );
    assert!(
        generated
            .warnings
            .iter()
            .any(|w| w.contains("petstore_auth"))
    );
    let upload = request(&generated, "uploadFile");
    assert_eq!(upload.stored.request.body, Body::None);
    assert!(
        generated
            .warnings
            .iter()
            .any(|w| w.contains("multipart/form-data"))
    );
    let find = request(&generated, "findPetsByStatus");
    assert_eq!(
        find.stored.request.url,
        "{{baseUrl}}/pet/findByStatus?status=available"
    );
}

#[test]
fn openapi_3_2_folders_methods_and_device_flow() {
    let generated = generate(&support::api("openapi32.yaml"));

    assert_eq!(
        generated.folders,
        [
            vec!["Search".to_owned()],
            vec!["Search".into(), "Advanced search".into()],
            vec!["untagged".into()]
        ]
    );
    let purge = request(&generated, "purgeIndex");
    assert_eq!(purge.stored.request.method, "PURGE");
    assert_eq!(
        purge.folder,
        ["Search", "Advanced search"],
        "the badge tag does not group"
    );
    let items = request(&generated, "listItems");
    assert_eq!(items.stored.request.url, "{{baseUrl}}/items?color=red");
    let search = request(&generated, "searchQuery");
    assert_eq!(search.stored.request.method, "QUERY");
    assert!(
        matches!(&search.stored.examples[0].body, Some(Body::Text { text, .. }) if text == r#"{"q":"whimbrel"}"#)
    );
    let Auth::OAuth2(config) = &generated.auth else {
        panic!("{:?}", generated.auth)
    };
    assert_eq!(config.grant, Grant::DeviceCode);
    assert_eq!(config.scope, "search");
    assert_eq!(config.client_id, "{{deviceClientId}}");
}

#[test]
fn install_writes_the_collection() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    wbl_format::collection::create(root, "Users").unwrap();
    let text = std::fs::read_to_string(support::fixture("openapi30.yaml")).unwrap();
    let generated = generate(&support::api("openapi30.yaml"));

    let source = wbl_openapi::Source {
        location: "openapi30.yaml".into(),
        text: text.clone(),
    };
    let installed = wbl_openapi::install(root, "", &generated, &source).unwrap();

    assert_eq!(
        installed.requests["getUser"],
        "users/get-a-user.req.toml".replace("get-a-user", "getuser")
    );
    let read = wbl_format::collection::read_request(root, &installed.requests["getUser"]).unwrap();
    assert_eq!(read.stored.variables["userId"], "42");
    let file = std::fs::read_to_string(root.join(&installed.requests["getUser"])).unwrap();
    assert!(
        file.ends_with("[openapi]\noperationId = \"getUser\"\n"),
        "{file}"
    );
    assert_eq!(installed.spec_dir, "openapi/users");
    assert_eq!(
        std::fs::read_to_string(root.join("openapi/users/source.yaml")).unwrap(),
        text
    );
    let manifest = std::fs::read_to_string(root.join("openapi/users/import.toml")).unwrap();
    assert!(
        manifest.contains("getUser = { file = \"users/getuser.req.toml\""),
        "{manifest}"
    );
    assert_eq!(installed.environments, ["production", "local"]);
    let kept = wbl_openapi::specs(root).unwrap();
    assert_eq!(kept[0].title, "Users");
    assert_eq!(kept[0].operations["getUser"], "users/getuser.req.toml");
    assert_eq!(
        wbl_openapi::source_file(root, "openapi/users").unwrap(),
        root.join("openapi/users/source.yaml")
    );
    assert!(wbl_openapi::source_file(root, "openapi/../x").is_none());
    assert_eq!(
        wbl_format::collection::auth::auth_of(root, "").unwrap(),
        generated.auth
    );
    let tree = wbl_format::collection::load(root).unwrap();
    assert_eq!(
        tree.children.len(),
        1,
        "one folder; openapi/ and environments/ are not folders"
    );

    let again = wbl_openapi::install(root, "", &generated, &source).unwrap();
    assert_eq!(again.spec_dir, "openapi/users-2");
    assert!(again.environments.is_empty());
    assert!(again.warnings.iter().any(|w| w.contains("Production")));
}
