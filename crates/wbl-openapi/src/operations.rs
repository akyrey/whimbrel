//! Paths and webhooks → operations: parameters (path-level merged in), bodies and responses
//! by media type, examples.

use std::collections::BTreeSet;

use serde_json::Value;
use url::Url;

use crate::build::{entries, flag, requirements, text};
use crate::model::{
    Callback, Link, MediaType, NamedExample, Operation, ParamLocation, Parameter, RequestBody,
    ResponseSpec,
};
use crate::refs::Resolver;

const METHODS: [&str; 9] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace", "query",
];

/// Operations of a `paths` (or `webhooks`) object, in document order.
pub fn paths<'a>(
    resolver: &mut Resolver<'a>,
    entry: &Url,
    value: Option<&'a Value>,
) -> Vec<Operation> {
    let Some(Value::Object(map)) = value else {
        return Vec::new();
    };
    let mut used = BTreeSet::new();
    let mut out = Vec::new();
    for (path, item) in map.iter().filter(|(k, _)| !k.starts_with("x-")) {
        let (base, item) = resolver.deref(entry, item);
        let item_params: Vec<&Value> = item
            .get("parameters")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect();
        let standard = METHODS
            .iter()
            .filter_map(|m| item.get(*m).map(|op| ((*m).to_owned(), op)));
        let standard: Vec<(String, &Value)> = standard.collect();
        let additional = entries(item, "additionalOperations").map(|(m, op)| (m.clone(), op));
        let additional: Vec<(String, &Value)> = additional.collect();
        for (method, op) in standard.into_iter().chain(additional) {
            let context = Context {
                base: &base,
                path,
                item,
                item_params: &item_params,
            };
            let mut operation = operation(resolver, &context, &method, op);
            operation.id = unique(&mut used, &operation.id);
            out.push(operation);
        }
    }
    out
}

struct Context<'c, 'v> {
    base: &'c Url,
    path: &'c str,
    item: &'v Value,
    item_params: &'c [&'v Value],
}

fn unique(used: &mut BTreeSet<String>, id: &str) -> String {
    let mut candidate = id.to_owned();
    let mut n = 2;
    while used.contains(&candidate) {
        candidate = format!("{id}-{n}");
        n += 1;
    }
    used.insert(candidate.clone());
    candidate
}

fn fallback_id(method: &str, path: &str) -> String {
    let words: Vec<String> = path
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    format!("{}-{}", method.to_ascii_lowercase(), words.join("-"))
        .trim_end_matches('-')
        .to_owned()
}

fn operation<'a>(
    resolver: &mut Resolver<'a>,
    cx: &Context<'_, 'a>,
    method: &str,
    op: &'a Value,
) -> Operation {
    let operation_id = text(op, "operationId");
    let own: Vec<Parameter> = op
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| parameter(resolver, cx.base, p))
        .collect();
    let inherited: Vec<Parameter> = cx
        .item_params
        .iter()
        .filter_map(|p| parameter(resolver, cx.base, p))
        .filter(|p| {
            !own.iter()
                .any(|o| o.name == p.name && o.location == p.location)
        })
        .collect();
    let request_body = op
        .get("requestBody")
        .map(|b| request_body(resolver, cx.base, b));
    let responses = entries(op, "responses")
        .map(|(status, r)| response(resolver, cx.base, status, r))
        .collect();
    Operation {
        id: operation_id
            .clone()
            .unwrap_or_else(|| fallback_id(method, cx.path)),
        has_operation_id: operation_id.is_some(),
        method: method.to_ascii_uppercase(),
        path: cx.path.to_owned(),
        summary: text(op, "summary").or_else(|| text(cx.item, "summary")),
        description: text(op, "description").or_else(|| text(cx.item, "description")),
        tags: op
            .get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        deprecated: flag(op, "deprecated"),
        parameters: inherited.into_iter().chain(own).collect(),
        request_body,
        responses,
        security: requirements(op.get("security")),
        callbacks: callbacks(resolver, cx.base, op),
    }
}

/// An operation's callbacks: each a map of URL expressions to path items.
fn callbacks<'a>(resolver: &mut Resolver<'a>, base: &Url, op: &'a Value) -> Vec<Callback> {
    entries(op, "callbacks")
        .map(|(name, value)| {
            let (at, value) = resolver.deref(base, value);
            Callback {
                name: name.clone(),
                operations: paths(resolver, &at, Some(value)),
            }
        })
        .collect()
}

fn links<'a>(resolver: &mut Resolver<'a>, base: &Url, response: &'a Value) -> Vec<Link> {
    entries(response, "links")
        .map(|(name, value)| {
            let (_, link) = resolver.deref(base, value);
            Link {
                name: name.clone(),
                operation_id: text(link, "operationId"),
                operation_ref: text(link, "operationRef"),
                parameters: entries(link, "parameters")
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            v.as_str().map_or_else(|| v.to_string(), str::to_owned),
                        )
                    })
                    .collect(),
                description: text(link, "description"),
            }
        })
        .collect()
}

fn location(text: &str) -> Option<ParamLocation> {
    Some(match text {
        "path" => ParamLocation::Path,
        "query" => ParamLocation::Query,
        "querystring" => ParamLocation::Querystring,
        "header" => ParamLocation::Header,
        "cookie" => ParamLocation::Cookie,
        _ => return None,
    })
}

fn parameter<'a>(resolver: &mut Resolver<'a>, base: &Url, value: &'a Value) -> Option<Parameter> {
    let (base, value) = resolver.deref(base, value);
    let location = location(value.get("in")?.as_str()?)?;
    // A parameter described by `content` (3.0+) takes its schema from the first media type.
    let content = media_types(resolver, &base, value.get("content"));
    let schema = value
        .get("schema")
        .map(|s| resolver.schema(&base, s))
        .or_else(|| content.first().and_then(|m| m.schema.clone()));
    let example = value
        .get("example")
        .cloned()
        .or_else(|| content.first().and_then(|m| m.example.clone()));
    let mut examples = examples(resolver, &base, value.get("examples"));
    if examples.is_empty() {
        examples = content
            .first()
            .map(|m| m.examples.clone())
            .unwrap_or_default();
    }
    Some(Parameter {
        name: text(value, "name")?,
        location,
        required: location == ParamLocation::Path || flag(value, "required"),
        deprecated: flag(value, "deprecated"),
        description: text(value, "description"),
        schema,
        example,
        examples,
        style: text(value, "style"),
        explode: value.get("explode").and_then(Value::as_bool),
    })
}

fn examples<'a>(
    resolver: &mut Resolver<'a>,
    base: &Url,
    value: Option<&'a Value>,
) -> Vec<NamedExample> {
    let Some(Value::Object(map)) = value else {
        return Vec::new();
    };
    map.iter()
        .map(|(name, example)| {
            let (_, example) = resolver.deref(base, example);
            NamedExample {
                name: name.clone(),
                summary: text(example, "summary"),
                value: example
                    .get("dataValue")
                    .or_else(|| example.get("value"))
                    .cloned(),
                serialized: text(example, "serializedValue"),
                external: text(example, "externalValue"),
            }
        })
        .collect()
}

fn media_types<'a>(
    resolver: &mut Resolver<'a>,
    base: &Url,
    value: Option<&'a Value>,
) -> Vec<MediaType> {
    let Some(Value::Object(map)) = value else {
        return Vec::new();
    };
    map.iter()
        .map(|(media_type, media)| {
            let (base, media) = resolver.deref(base, media);
            MediaType {
                media_type: media_type.clone(),
                schema: media.get("schema").map(|s| resolver.schema(&base, s)),
                item_schema: media.get("itemSchema").map(|s| resolver.schema(&base, s)),
                example: media.get("example").cloned(),
                examples: examples(resolver, &base, media.get("examples")),
            }
        })
        .collect()
}

fn request_body<'a>(resolver: &mut Resolver<'a>, base: &Url, value: &'a Value) -> RequestBody {
    let (base, value) = resolver.deref(base, value);
    RequestBody {
        required: flag(value, "required"),
        description: text(value, "description"),
        content: media_types(resolver, &base, value.get("content")),
    }
}

fn response<'a>(
    resolver: &mut Resolver<'a>,
    base: &Url,
    status: &str,
    value: &'a Value,
) -> ResponseSpec {
    let (base, value) = resolver.deref(base, value);
    let description = text(value, "description").filter(|d| !d.is_empty());
    ResponseSpec {
        status: status.to_owned(),
        summary: text(value, "summary").or_else(|| {
            description
                .as_ref()
                .and_then(|d| d.lines().next())
                .map(str::to_owned)
        }),
        description,
        content: media_types(resolver, &base, value.get("content")),
        links: links(resolver, &base, value),
    }
}
