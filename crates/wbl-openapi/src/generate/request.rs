//! One operation → one request with its saved examples.

use std::collections::BTreeMap;

use serde_json::Value;
use wbl_core::example::{Example, Param};
use wbl_core::{Body, BodyKind, Header, Request};
use wbl_format::StoredRequest;
use wbl_format::collection::slug;

use super::values::{
    Wire, body_text, body_value, example_text, param_value, preferred, query_pairs, query_string,
    text_of, wire,
};
use crate::model::{Api, MediaType, Operation, ParamLocation, Parameter};

/// Header parameters OAS says to ignore: the request sets these itself.
const IGNORED_HEADERS: [&str; 3] = ["accept", "content-type", "authorization"];
/// The saved example holding every optional query parameter.
const ALL_PARAMETERS: &str = "All parameters";
const ORDER_STEP: i64 = 10;

pub fn name_of(op: &Operation) -> String {
    op.summary
        .clone()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.lines().next().unwrap_or("").trim().to_owned())
        .or_else(|| op.has_operation_id.then(|| op.id.clone()))
        .unwrap_or_else(|| format!("{} {}", op.method, op.path))
}

/// `/users/{id}` → `{{baseUrl}}/users/{{id}}`.
fn url_of(path: &str) -> String {
    let templated = path.replace('{', "{{").replace('}', "}}");
    format!("{{{{baseUrl}}}}{templated}")
}

fn kind_of(wire: Wire) -> BodyKind {
    match wire {
        Wire::Json => BodyKind::Json,
        Wire::Xml => BodyKind::Xml,
        Wire::Form | Wire::Text => BodyKind::Text,
    }
}

struct Parts<'a> {
    api: &'a Api,
    op: &'a Operation,
    media: Option<(&'a MediaType, Wire)>,
}

impl Parts<'_> {
    fn params(&self, location: ParamLocation) -> impl Iterator<Item = &Parameter> {
        self.op
            .parameters
            .iter()
            .filter(move |p| p.location == location)
    }

    fn value(&self, param: &Parameter) -> Value {
        param_value(param, &self.api.schemas)
    }

    fn url(&self, optional: bool) -> String {
        let mut url = url_of(&self.op.path);
        let pairs: Vec<(String, String)> = self
            .params(ParamLocation::Query)
            .filter(|p| p.required || optional)
            .flat_map(|p| query_pairs(p, &self.value(p)))
            .collect();
        let whole = self.params(ParamLocation::Querystring).next().map(|p| {
            p.examples
                .iter()
                .find_map(|e| e.serialized.clone())
                .unwrap_or_else(|| super::values::form(&self.value(p)))
        });
        let query: Vec<String> = [Some(query_string(&pairs)), whole]
            .into_iter()
            .flatten()
            .filter(|q| !q.is_empty())
            .collect();
        if !query.is_empty() {
            url = format!("{url}?{}", query.join("&"));
        }
        url
    }

    fn headers(&self) -> Vec<Header> {
        let mut headers: Vec<Header> = self
            .params(ParamLocation::Header)
            .filter(|p| !IGNORED_HEADERS.contains(&p.name.to_ascii_lowercase().as_str()))
            .map(|p| Header {
                name: p.name.clone(),
                value: text_of(&self.value(p)),
                enabled: p.required,
            })
            .collect();
        let cookies: Vec<&Parameter> = self.params(ParamLocation::Cookie).collect();
        if !cookies.is_empty() {
            let value = cookies
                .iter()
                .map(|p| format!("{}={}", p.name, text_of(&self.value(p))))
                .collect::<Vec<_>>()
                .join("; ");
            headers.push(Header {
                name: "Cookie".into(),
                value,
                enabled: cookies.iter().any(|p| p.required),
            });
        }
        if let Some((media, _)) = self.media {
            headers.push(Header {
                name: "Content-Type".into(),
                value: media.media_type.clone(),
                enabled: true,
            });
        }
        headers
    }

    fn body(&self) -> Body {
        match self.media {
            Some((media, wire)) => Body::Text {
                kind: kind_of(wire),
                text: body_value(media, &self.api.schemas)
                    .map(|v| body_text(wire, &v))
                    .unwrap_or_default(),
            },
            None => Body::None,
        }
    }

    fn variables(&self) -> BTreeMap<String, String> {
        self.params(ParamLocation::Path)
            .map(|p| (p.name.clone(), text_of(&self.value(p))))
            .collect()
    }
}

/// Names of the examples on the body and the parameters, in first-seen order.
fn example_names(parts: &Parts<'_>) -> Vec<String> {
    let body = parts
        .media
        .into_iter()
        .flat_map(|(m, _)| m.examples.iter().map(|e| e.name.clone()));
    let params = parts
        .op
        .parameters
        .iter()
        .flat_map(|p| p.examples.iter().map(|e| e.name.clone()));
    let mut names: Vec<String> = Vec::new();
    for name in body.chain(params) {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn named_example(parts: &Parts<'_>, name: &str, order: i64) -> Example {
    let mut example = Example {
        order,
        ..Example::named(&slug(name), name)
    };
    for param in &parts.op.parameters {
        let Some(value) = param
            .examples
            .iter()
            .find(|e| e.name == name)
            .and_then(|e| e.value.clone())
        else {
            continue;
        };
        match param.location {
            ParamLocation::Path => {
                example
                    .variables
                    .insert(param.name.clone(), text_of(&value));
            }
            ParamLocation::Query => {
                example
                    .query
                    .extend(
                        query_pairs(param, &value)
                            .into_iter()
                            .map(|(name, value)| Param {
                                name,
                                value,
                                enabled: true,
                            }),
                    )
            }
            ParamLocation::Header => example.headers.push(Header {
                name: param.name.clone(),
                value: text_of(&value),
                enabled: true,
            }),
            ParamLocation::Querystring | ParamLocation::Cookie => {}
        }
    }
    if let Some((media, wire)) = parts.media
        && let Some(text) = media
            .examples
            .iter()
            .find(|e| e.name == name)
            .and_then(|e| example_text(wire, e))
    {
        example.body = Some(Body::Text {
            kind: kind_of(wire),
            text,
        });
    }
    example
}

fn all_parameters(parts: &Parts<'_>, order: i64) -> Option<Example> {
    let optional: Vec<Param> = parts
        .params(ParamLocation::Query)
        .filter(|p| !p.required)
        .flat_map(|p| query_pairs(p, &parts.value(p)))
        .map(|(name, value)| Param {
            name,
            value,
            enabled: true,
        })
        .collect();
    (!optional.is_empty()).then(|| Example {
        order,
        query: optional,
        ..Example::named("all-parameters", ALL_PARAMETERS)
    })
}

/// The request for `op`; `warnings` gets what could not be generated.
pub fn request(api: &Api, op: &Operation, warnings: &mut Vec<String>) -> StoredRequest {
    let content = op
        .request_body
        .as_ref()
        .map(|b| b.content.as_slice())
        .unwrap_or_default();
    let media = preferred(content).and_then(|m| wire(&m.media_type).map(|w| (m, w)));
    if media.is_none()
        && let Some(m) = content.first()
    {
        warnings.push(format!(
            "{}: a {} body is not generated; add it by hand",
            op.id, m.media_type
        ));
    }
    let parts = Parts { api, op, media };
    let names = example_names(&parts);
    let mut examples: Vec<Example> = names
        .iter()
        .zip(1..)
        .map(|(n, i)| named_example(&parts, n, i * ORDER_STEP))
        .collect();
    let next = i64::try_from(examples.len() + 1).unwrap_or(i64::MAX / ORDER_STEP) * ORDER_STEP;
    examples.extend(all_parameters(&parts, next));
    StoredRequest {
        name: name_of(op),
        order: 0,
        request: Request {
            method: op.method.clone(),
            url: parts.url(false),
            headers: parts.headers(),
            body: parts.body(),
        },
        examples,
        settings: wbl_core::RequestSettings::default(),
        auth: wbl_core::auth::Auth::Inherit,
        variables: parts.variables(),
        captures: Vec::new(),
        assertions: Vec::new(),
    }
}
