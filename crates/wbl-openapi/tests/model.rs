#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tasks 6.1–6.3: every version reads into the one model.

mod support;

use serde_json::json;
use wbl_openapi::{ParamLocation, SchemeKind, SpecVersion, detect, parse};

#[test]
fn versions_are_detected() {
    let v = |text: &str| detect(&parse(text).unwrap());
    assert_eq!(v("swagger: '2.0'").unwrap().0, SpecVersion::Swagger2);
    assert_eq!(v("openapi: 3.0.4").unwrap().0, SpecVersion::OpenApi30);
    assert_eq!(v("openapi: 3.1.2").unwrap().0, SpecVersion::OpenApi31);
    assert_eq!(v("openapi: 3.2.0").unwrap().0, SpecVersion::OpenApi32);
    let (newer, _, warning) = v("openapi: 3.3.0").unwrap();
    assert_eq!(newer, SpecVersion::OpenApi32);
    assert!(warning.unwrap().contains("3.3.0"));
    assert_eq!(
        v("openapi: 3.1").unwrap().0,
        SpecVersion::OpenApi31,
        "a YAML number"
    );
    assert_eq!(v("openapi: 4.0.0").unwrap_err().code(), "openapi_unknown");
    assert_eq!(v("swagger: '1.2'").unwrap_err().code(), "openapi_unknown");
    assert_eq!(v("title: nope").unwrap_err().code(), "openapi_unknown");
}

#[test]
fn swagger_2_is_upconverted() {
    let api = support::api("swagger2.yaml");

    assert_eq!(api.version, SpecVersion::Swagger2);
    assert_eq!(api.api_version, "1.0.6");
    let urls: Vec<_> = api.servers.iter().map(|s| s.url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://petstore.swagger.io/v2",
            "http://petstore.swagger.io/v2"
        ]
    );

    let add = api.operations.iter().find(|o| o.id == "addPet").unwrap();
    let body = add.request_body.as_ref().unwrap();
    assert!(body.required);
    assert_eq!(body.content[0].media_type, "application/json");
    assert_eq!(
        body.content[0].schema,
        Some(json!({"$ref": "#/schemas/Pet"}))
    );

    let find = api
        .operations
        .iter()
        .find(|o| o.id == "findPetsByStatus")
        .unwrap();
    let status = &find.parameters[0];
    assert_eq!(
        (status.style.as_deref(), status.explode),
        (Some("form"), Some(true))
    );
    assert_eq!(
        status.schema.as_ref().unwrap()["items"]["enum"][0],
        "available"
    );

    let upload = api
        .operations
        .iter()
        .find(|o| o.id == "uploadFile")
        .unwrap();
    assert_eq!(
        upload.parameters[0].name, "petId",
        "path-level $ref parameter"
    );
    assert_eq!(upload.parameters[0].location, ParamLocation::Path);
    let form = &upload.request_body.as_ref().unwrap().content[0];
    assert_eq!(form.media_type, "multipart/form-data");
    assert_eq!(
        form.schema.as_ref().unwrap()["properties"]["file"]["format"],
        "binary"
    );
    assert_eq!(form.schema.as_ref().unwrap()["required"], json!(["file"]));
    assert_eq!(upload.responses[0].description.as_deref(), Some("All good"));

    let inventory = api
        .operations
        .iter()
        .find(|o| o.id == "getInventory")
        .unwrap();
    assert_eq!(
        inventory.responses[0].content[0].example,
        Some(json!({"available": 3}))
    );
    assert_eq!(inventory.parameters[0].style.as_deref(), Some("form"));

    let oauth = api
        .security_schemes
        .iter()
        .find(|s| s.name == "petstore_auth")
        .unwrap();
    let SchemeKind::OAuth2 { flows, .. } = &oauth.kind else {
        panic!("{oauth:?}")
    };
    assert_eq!(flows[0].kind, "implicit");
    assert!(
        matches!(&api.security_schemes.iter().find(|s| s.name == "basic").unwrap().kind, SchemeKind::Http { scheme, .. } if scheme == "basic")
    );
    assert_eq!(
        api.schemas["Pet"]["properties"]["name"]["examples"],
        json!(["doggie"])
    );
    assert_eq!(
        api.schemas["Pet"]["properties"]["category"],
        json!({"$ref": "#/schemas/Category"})
    );
}

#[test]
fn openapi_3_0_reads_with_shared_parts_and_nullable() {
    let api = support::api("openapi30.yaml");

    assert_eq!(api.api_version, "2");
    assert_eq!(api.servers[0].variables[0].name, "region");
    assert_eq!(api.servers[0].variables[0].values, ["eu", "us"]);
    let list = &api.operations[0];
    assert_eq!(
        (list.id.as_str(), list.method.as_str()),
        ("listUsers", "GET")
    );
    let locations: Vec<_> = list.parameters.iter().map(|p| p.location).collect();
    assert_eq!(
        locations,
        [
            ParamLocation::Query,
            ParamLocation::Query,
            ParamLocation::Query,
            ParamLocation::Header,
            ParamLocation::Cookie
        ]
    );

    let create = api
        .operations
        .iter()
        .find(|o| o.id == "createUser")
        .unwrap();
    let examples = &create.request_body.as_ref().unwrap().content[0].examples;
    assert_eq!(
        examples.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["admin", "member"]
    );
    assert_eq!(examples[0].summary.as_deref(), Some("An admin"));

    let get = api.operations.iter().find(|o| o.id == "getUser").unwrap();
    assert!(get.parameters[0].required);
    assert_eq!(get.parameters[0].example, Some(json!(42)));
    assert_eq!(get.responses[1].status, "default");
    assert_eq!(
        get.responses[1].summary.as_deref(),
        Some("Something went wrong")
    );

    let nickname = &api.schemas["User"]["allOf"][1]["properties"]["nickname"];
    assert_eq!(nickname["type"], json!(["string", "null"]));
    assert_eq!(api.security[0].uses[0].scheme, "bearer");
    // Team ↔ User is a cycle: both kept once, as links.
    assert_eq!(
        api.schemas["Team"]["properties"]["members"]["items"],
        json!({"$ref": "#/schemas/User"})
    );
}

#[test]
fn openapi_3_1_keeps_2020_12_and_webhooks() {
    let api = support::api("openapi31.yaml");

    assert_eq!(api.webhooks.len(), 1);
    assert_eq!(api.webhooks[0].path, "newEvent");
    assert_eq!(
        api.webhooks[0].summary.as_deref(),
        Some("A new event happened")
    );
    let put = api.operations.iter().find(|o| o.id == "putEvent").unwrap();
    let schema = put.request_body.as_ref().unwrap().content[0]
        .schema
        .clone()
        .unwrap();
    assert_eq!(
        schema,
        json!({"$ref": "#/schemas/Event", "description": "The new event"}),
        "$ref siblings stay"
    );
    let event = &api.schemas["Event"];
    assert_eq!(
        event["properties"]["parent"],
        json!({"$ref": "#/schemas/Event"})
    );
    assert_eq!(
        event["properties"]["payload"],
        json!({"$ref": "#/schemas/Payload"})
    );
    assert_eq!(api.schemas["Payload"]["properties"]["size"]["default"], 7);
    assert_eq!(
        event["properties"]["note"]["type"],
        json!(["string", "null"])
    );
}

#[test]
fn openapi_3_2_features() {
    let api = support::api("openapi32.yaml");

    let methods: Vec<_> = api
        .operations
        .iter()
        .map(|o| (o.id.as_str(), o.method.as_str()))
        .collect();
    assert_eq!(
        methods,
        [
            ("searchQuery", "QUERY"),
            ("purgeIndex", "PURGE"),
            ("listItems", "GET"),
            ("get-untagged-things", "GET")
        ]
    );
    assert_eq!(api.tags[1].parent.as_deref(), Some("search"));
    assert_eq!(api.tags[2].kind.as_deref(), Some("badge"));

    let search = &api.operations[0];
    let example = &search.request_body.as_ref().unwrap().content[0].examples[0];
    assert_eq!(example.value, Some(json!({"q": "whimbrel"})));
    assert_eq!(example.serialized.as_deref(), Some(r#"{"q":"whimbrel"}"#));
    assert_eq!(search.responses[0].summary.as_deref(), Some("The matches"));
    assert!(search.responses[0].content[0].item_schema.is_some());

    let filter = &api.operations[2].parameters[0];
    assert_eq!(filter.location, ParamLocation::Querystring);
    assert_eq!(
        filter.schema.as_ref().unwrap()["properties"]["size"]["type"],
        "integer"
    );
    assert_eq!(filter.examples[0].serialized.as_deref(), Some("color=red"));

    let device = &api.security_schemes[0];
    let SchemeKind::OAuth2 {
        flows,
        metadata_url,
    } = &device.kind
    else {
        panic!()
    };
    assert_eq!(flows[0].kind, "device_code");
    assert_eq!(
        flows[0].device_url.as_deref(),
        Some("https://auth.example.com/device")
    );
    assert!(metadata_url.is_some());
    assert!(api.security_schemes[1].deprecated);
}

#[test]
fn references_across_files_and_cycles() {
    let api = support::api("split/main.yaml");

    assert_eq!(api.operations[0].id, "listOrders");
    assert_eq!(
        api.operations[0].responses[0].content[0].schema,
        Some(json!({"type": "array", "items": {"$ref": "#/schemas/Order"}}))
    );
    assert_eq!(
        api.schemas["Order"]["properties"]["lines"]["items"],
        json!({"$ref": "#/schemas/Line"})
    );
    assert_eq!(
        api.schemas["Line"]["properties"]["order"],
        json!({"$ref": "#/schemas/Order"})
    );
}

#[test]
fn broken_references_warn() {
    let doc = parse("openapi: 3.1.0\ninfo: {title: t, version: '1'}\npaths:\n  /a:\n    $ref: '#/nowhere'\n  /b:\n    get:\n      responses:\n        '200': {description: ok, content: {application/json: {schema: {$ref: '#/components/schemas/Gone'}}}}\n").unwrap();
    let docs = wbl_openapi::Documents {
        entry: Some(url::Url::parse("file:///spec.yaml").unwrap()),
        by_url: [("file:///spec.yaml".to_owned(), doc)].into(),
    };

    let api = wbl_openapi::build(docs).unwrap();

    assert_eq!(api.operations.len(), 1);
    assert_eq!(api.warnings.len(), 2, "{:?}", api.warnings);
}

#[test]
fn links_and_callbacks_are_read() {
    let api = support::api("openapi30.yaml");
    let create = api
        .operations
        .iter()
        .find(|o| o.id == "createUser")
        .unwrap();

    let link = &create.responses[0].links[0];
    assert_eq!(link.name, "GetUser");
    assert_eq!(link.operation_id.as_deref(), Some("getUser"));
    assert_eq!(
        link.parameters,
        [("userId".to_owned(), "$response.body#/id".to_owned())]
    );
    let callback = &create.callbacks[0];
    assert_eq!(callback.name, "onCreated");
    assert_eq!(callback.operations[0].path, "{$request.body#/callbackUrl}");
    assert_eq!(callback.operations[0].method, "POST");
}
