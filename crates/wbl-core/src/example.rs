//! Saved examples (FR-2.8, D-H08): named overlays on a request. Applying one never changes the
//! request; fields the example does not mention come from the request.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::interpolate::{Scope, Value};
use crate::{Body, Header, Request};

/// A query parameter, matched by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Param {
    pub name: String,
    pub value: String,
    /// `false` removes the parameter from the request.
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Example {
    /// Stable; tabs and runner steps refer to examples by id.
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub order: i64,
    /// Variables of the highest precedence while the example is used.
    pub variables: BTreeMap<String, String>,
    pub query: Vec<Param>,
    /// Matched by name, ignoring case; a disabled one turns the request's header off.
    pub headers: Vec<Header>,
    /// Replaces the whole body when present.
    pub body: Option<Body>,
}

impl Example {
    pub fn named(id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            order: 0,
            variables: BTreeMap::new(),
            query: Vec::new(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// The example's variables, as the first scope of resolution.
    pub fn scope(&self) -> Scope {
        Scope::new(
            self.variables
                .iter()
                .map(|(k, v)| (k.clone(), Value::Plain(v.clone())))
                .collect(),
        )
    }
}

/// `url` split into what comes before the query, the query's pairs, and the fragment.
struct Parts<'a> {
    head: &'a str,
    pairs: Vec<(String, String)>,
    fragment: Option<&'a str>,
}

fn split(url: &str) -> Parts<'_> {
    let (rest, fragment) = match url.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment)),
        None => (url, None),
    };
    let (head, query) = rest.split_once('?').unwrap_or((rest, ""));
    let pairs = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (name, value) = p.split_once('=').unwrap_or((p, ""));
            (name.to_owned(), value.to_owned())
        })
        .collect();
    Parts {
        head,
        pairs,
        fragment,
    }
}

fn join(parts: &Parts<'_>) -> String {
    let mut url = parts.head.to_owned();
    if !parts.pairs.is_empty() {
        let query: Vec<String> = parts
            .pairs
            .iter()
            .map(|(n, v)| format!("{n}={v}"))
            .collect();
        url.push('?');
        url.push_str(&query.join("&"));
    }
    if let Some(fragment) = parts.fragment {
        url.push('#');
        url.push_str(fragment);
    }
    url
}

fn apply_query(url: &str, params: &[Param]) -> String {
    if params.is_empty() {
        return url.to_owned();
    }
    let mut parts = split(url);
    for param in params {
        if !param.enabled {
            parts.pairs.retain(|(n, _)| n != &param.name);
        } else if let Some(pair) = parts.pairs.iter_mut().find(|(n, _)| n == &param.name) {
            pair.1 = param.value.clone();
        } else {
            parts.pairs.push((param.name.clone(), param.value.clone()));
        }
    }
    join(&parts)
}

fn apply_headers(headers: &[Header], overlay: &[Header]) -> Vec<Header> {
    let mut out = headers.to_vec();
    for header in overlay {
        let matches = |h: &Header| h.name.eq_ignore_ascii_case(&header.name);
        if !header.enabled {
            out.iter_mut()
                .filter(|h| matches(h))
                .for_each(|h| h.enabled = false);
        } else if let Some(existing) = out.iter_mut().find(|h| matches(h)) {
            existing.value = header.value.clone();
            existing.enabled = true;
        } else {
            out.push(header.clone());
        }
    }
    out
}

/// `request` with `example` laid over it.
pub fn apply(request: &Request, example: &Example) -> Request {
    Request {
        method: request.method.clone(),
        url: apply_query(&request.url, &example.query),
        headers: apply_headers(&request.headers, &example.headers),
        body: example.body.clone().unwrap_or_else(|| request.body.clone()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "An example keeps the request's method and path; change those in the request, or use a variable"
)]
pub struct OverlayError;

fn query_diff(base: &[(String, String)], draft: &[(String, String)]) -> Vec<Param> {
    let mut params: Vec<Param> = draft
        .iter()
        .filter(|pair| !base.contains(pair))
        .map(|(n, v)| Param {
            name: n.clone(),
            value: v.clone(),
            enabled: true,
        })
        .collect();
    params.extend(
        base.iter()
            .filter(|(n, _)| !draft.iter().any(|(d, _)| d == n))
            .map(|(n, v)| Param {
                name: n.clone(),
                value: v.clone(),
                enabled: false,
            }),
    );
    params
}

fn header_diff(base: &[Header], draft: &[Header]) -> Vec<Header> {
    let mut headers: Vec<Header> = draft
        .iter()
        .filter(|h| !base.contains(h))
        .cloned()
        .collect();
    headers.extend(
        base.iter()
            .filter(|b| b.enabled && !draft.iter().any(|d| d.name.eq_ignore_ascii_case(&b.name)))
            .map(|b| Header {
                enabled: false,
                ..b.clone()
            }),
    );
    headers
}

/// The example that turns `base` into `draft` ("save the current edits as an example").
pub fn overlay_of(
    base: &Request,
    draft: &Request,
    id: &str,
    name: &str,
) -> Result<Example, OverlayError> {
    let (from, to) = (split(&base.url), split(&draft.url));
    if base.method != draft.method || from.head != to.head || from.fragment != to.fragment {
        return Err(OverlayError);
    }
    Ok(Example {
        query: query_diff(&from.pairs, &to.pairs),
        headers: header_diff(&base.headers, &draft.headers),
        body: (base.body != draft.body).then(|| draft.body.clone()),
        ..Example::named(id, name)
    })
}

#[cfg(test)]
mod tests;
