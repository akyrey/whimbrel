//! Swagger 2.0 → OpenAPI 3.0 shape (OPENAPI.md "Upconversion"), so one builder reads both.

use serde_json::{Map, Value, json};
use url::Url;

const METHODS: [&str; 7] = ["get", "put", "post", "delete", "options", "head", "patch"];
const DEFAULT_MEDIA: &str = "application/json";
/// Fields of a 2.0 non-body parameter that are schema keywords in 3.0.
const SCHEMA_KEYS: [&str; 16] = [
    "type",
    "format",
    "items",
    "enum",
    "default",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "minLength",
    "maxLength",
    "pattern",
    "minItems",
    "maxItems",
    "uniqueItems",
    "multipleOf",
];

fn get<'v>(value: &'v Value, key: &str) -> Option<&'v Value> {
    value.get(key)
}

fn strings(value: Option<&Value>) -> Option<Vec<String>> {
    value.and_then(Value::as_array).map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    })
}

/// Every `$ref` to a 2.0 section rewritten to its 3.0 place.
pub fn rewrite_refs(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| match (k.as_str(), v) {
                    ("$ref", Value::String(r)) => (k.clone(), Value::String(rewrite_ref(r))),
                    _ => (k.clone(), rewrite_refs(v)),
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(rewrite_refs).collect()),
        other => other.clone(),
    }
}

fn rewrite_ref(reference: &str) -> String {
    reference.replace("#/definitions/", "#/components/schemas/")
}

/// A local `#/parameters/x` or `#/responses/x` reference followed (they change shape, so they
/// are inlined rather than kept).
fn local<'v>(doc: &'v Value, value: &'v Value) -> &'v Value {
    let Some(reference) = value.get("$ref").and_then(Value::as_str) else {
        return value;
    };
    let Some(path) = reference.strip_prefix("#/") else {
        return value;
    };
    path.split('/')
        .try_fold(doc, |at, key| {
            at.get(key.replace("~1", "/").replace("~0", "~"))
        })
        .unwrap_or(value)
}

fn servers(doc: &Value, entry: Option<&Url>) -> Value {
    let host = get(doc, "host")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            entry
                .filter(|u| matches!(u.scheme(), "http" | "https"))
                .and_then(|u| {
                    u.host_str().map(|h| match u.port() {
                        Some(p) => format!("{h}:{p}"),
                        None => h.to_owned(),
                    })
                })
        });
    let base_path = get(doc, "basePath").and_then(Value::as_str).unwrap_or("");
    let schemes = strings(get(doc, "schemes"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| vec!["https".into()]);
    match host {
        Some(host) => Value::Array(
            schemes
                .iter()
                .map(|s| json!({ "url": format!("{s}://{host}{base_path}") }))
                .collect(),
        ),
        None if !base_path.is_empty() => json!([{ "url": base_path }]),
        None => json!([]),
    }
}

fn style_of(format: &str, location: &str) -> Option<(&'static str, bool)> {
    match format {
        "csv" => Some(if location == "query" {
            ("form", false)
        } else {
            ("simple", false)
        }),
        "ssv" => Some(("spaceDelimited", false)),
        "pipes" => Some(("pipeDelimited", false)),
        "multi" => Some(("form", true)),
        _ => None,
    }
}

fn parameter(param: &Value) -> Value {
    let location = get(param, "in").and_then(Value::as_str).unwrap_or("query");
    let schema: Map<String, Value> = SCHEMA_KEYS
        .iter()
        .filter_map(|k| param.get(*k).map(|v| ((*k).to_owned(), v.clone())))
        .collect();
    let mut out = Map::new();
    for key in ["name", "in", "description", "required"] {
        if let Some(v) = param.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    if let Some(example) = param.get("x-example") {
        out.insert("example".into(), example.clone());
    }
    out.insert("schema".into(), Value::Object(schema));
    if let Some((style, explode)) = param
        .get("collectionFormat")
        .and_then(Value::as_str)
        .and_then(|f| style_of(f, location))
    {
        out.insert("style".into(), json!(style));
        out.insert("explode".into(), json!(explode));
    }
    Value::Object(out)
}

fn form_body(params: &[&Value], consumes: &[String]) -> Value {
    let has_file = params
        .iter()
        .any(|p| p.get("type").and_then(Value::as_str) == Some("file"));
    let media = if has_file || consumes.iter().any(|c| c == "multipart/form-data") {
        "multipart/form-data"
    } else {
        "application/x-www-form-urlencoded"
    };
    let mut properties = Map::new();
    let mut required = Vec::new();
    for p in params {
        let Some(name) = p.get("name").and_then(Value::as_str) else {
            continue;
        };
        let mut schema = parameter(p)
            .get("schema")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if schema.get("type").and_then(Value::as_str) == Some("file") {
            schema = json!({ "type": "string", "format": "binary" });
        }
        if let (Some(d), Value::Object(s)) = (p.get("description"), &mut schema) {
            s.insert("description".into(), d.clone());
        }
        properties.insert(name.to_owned(), schema);
        if p.get("required") == Some(&Value::Bool(true)) {
            required.push(json!(name));
        }
    }
    json!({ "content": { media: { "schema": { "type": "object", "properties": properties, "required": required } } } })
}

fn body(param: &Value, consumes: &[String]) -> Value {
    let schema = param.get("schema").cloned().unwrap_or_else(|| json!({}));
    let content: Map<String, Value> = consumes
        .iter()
        .map(|m| (m.clone(), json!({ "schema": schema })))
        .collect();
    let mut out = json!({ "content": content, "required": param.get("required").cloned().unwrap_or(json!(false)) });
    if let (Some(d), Value::Object(o)) = (param.get("description"), &mut out) {
        o.insert("description".into(), d.clone());
    }
    out
}

fn response(doc: &Value, value: &Value, produces: &[String]) -> Value {
    let value = local(doc, value);
    let mut out = Map::new();
    out.insert(
        "description".into(),
        value.get("description").cloned().unwrap_or(json!("")),
    );
    if let Some(schema) = value.get("schema") {
        let examples = value.get("examples");
        let content: Map<String, Value> = produces
            .iter()
            .map(|m| {
                let mut media = json!({ "schema": schema });
                if let (Some(example), Value::Object(o)) =
                    (examples.and_then(|e| e.get(m)), &mut media)
                {
                    o.insert("example".into(), example.clone());
                }
                (m.clone(), media)
            })
            .collect();
        out.insert("content".into(), Value::Object(content));
    }
    Value::Object(out)
}

struct Context<'a> {
    doc: &'a Value,
    consumes: Vec<String>,
    produces: Vec<String>,
}

fn operation(cx: &Context<'_>, op: &Value, path_params: &[Value]) -> Value {
    let consumes = strings(op.get("consumes"))
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| cx.consumes.clone());
    let produces = strings(op.get("produces"))
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| cx.produces.clone());
    let own: Vec<&Value> = op
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|p| local(cx.doc, p))
        .collect();
    let key = |p: &Value| (p.get("name").cloned(), p.get("in").cloned());
    let inherited = path_params
        .iter()
        .map(|p| local(cx.doc, p))
        .filter(|p| !own.iter().any(|o| key(o) == key(p)));
    let all: Vec<&Value> = inherited.chain(own.iter().copied()).collect();
    let is = |p: &&Value, location: &str| p.get("in").and_then(Value::as_str) == Some(location);
    let mut out: Map<String, Value> = [
        "operationId",
        "summary",
        "description",
        "tags",
        "deprecated",
        "security",
    ]
    .iter()
    .filter_map(|k| op.get(*k).map(|v| ((*k).to_owned(), v.clone())))
    .collect();
    let params: Vec<Value> = all
        .iter()
        .filter(|p| !is(p, "body") && !is(p, "formData"))
        .map(|p| parameter(p))
        .collect();
    out.insert("parameters".into(), Value::Array(params));
    let form: Vec<&Value> = all.iter().filter(|p| is(p, "formData")).copied().collect();
    if let Some(b) = all.iter().find(|p| is(p, "body")) {
        out.insert("requestBody".into(), body(b, &consumes));
    } else if !form.is_empty() {
        out.insert("requestBody".into(), form_body(&form, &consumes));
    }
    let responses: Map<String, Value> = op
        .get("responses")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(k, _)| !k.starts_with("x-"))
        .map(|(k, v)| (k.clone(), response(cx.doc, v, &produces)))
        .collect();
    out.insert("responses".into(), Value::Object(responses));
    Value::Object(out)
}

fn flows(scheme: &Value) -> Value {
    let (name, auth, token) = match scheme.get("flow").and_then(Value::as_str).unwrap_or("") {
        "implicit" => ("implicit", true, false),
        "password" => ("password", false, true),
        "application" => ("clientCredentials", false, true),
        _ => ("authorizationCode", true, true),
    };
    let mut flow = Map::new();
    if auth {
        flow.insert(
            "authorizationUrl".into(),
            scheme.get("authorizationUrl").cloned().unwrap_or(json!("")),
        );
    }
    if token {
        flow.insert(
            "tokenUrl".into(),
            scheme.get("tokenUrl").cloned().unwrap_or(json!("")),
        );
    }
    flow.insert(
        "scopes".into(),
        scheme.get("scopes").cloned().unwrap_or(json!({})),
    );
    json!({ name: flow })
}

fn security_scheme(scheme: &Value) -> Value {
    let description = scheme.get("description").cloned().unwrap_or(Value::Null);
    let mut out = match scheme.get("type").and_then(Value::as_str).unwrap_or("") {
        "basic" => json!({ "type": "http", "scheme": "basic" }),
        "apiKey" => json!({ "type": "apiKey", "name": scheme.get("name"), "in": scheme.get("in") }),
        "oauth2" => json!({ "type": "oauth2", "flows": flows(scheme) }),
        other => json!({ "type": other }),
    };
    if let (false, Value::Object(o)) = (description.is_null(), &mut out) {
        o.insert("description".into(), description);
    }
    out
}

fn paths(cx: &Context<'_>) -> Value {
    let mut out = Map::new();
    for (path, item) in cx
        .doc
        .get("paths")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if path.starts_with("x-") {
            continue;
        }
        let path_params: Vec<Value> = item
            .get("parameters")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ops: Map<String, Value> = METHODS
            .iter()
            .filter_map(|m| {
                item.get(*m)
                    .map(|op| ((*m).to_owned(), operation(cx, op, &path_params)))
            })
            .collect();
        out.insert(path.clone(), Value::Object(ops));
    }
    Value::Object(out)
}

/// The 3.0 document a 2.0 document means. `entry` supplies the host when the document has none.
pub fn upconvert(doc: &Value, entry: Option<&Url>) -> Value {
    let media = |key| {
        strings(doc.get(key))
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| vec![DEFAULT_MEDIA.into()])
    };
    let cx = Context {
        doc,
        consumes: media("consumes"),
        produces: media("produces"),
    };
    let schemes: Map<String, Value> = doc
        .get("securityDefinitions")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(k, v)| (k.clone(), security_scheme(v)))
        .collect();
    let mut out = json!({
        "openapi": "3.0.3",
        "info": doc.get("info").cloned().unwrap_or(json!({})),
        "servers": servers(doc, entry),
        "paths": paths(&cx),
        "components": {
            "schemas": doc.get("definitions").cloned().unwrap_or(json!({})),
            "securitySchemes": schemes,
        },
    });
    if let Value::Object(o) = &mut out {
        for key in ["tags", "security", "externalDocs"] {
            if let Some(v) = doc.get(key) {
                o.insert(key.into(), v.clone());
            }
        }
    }
    rewrite_refs(&out)
}
