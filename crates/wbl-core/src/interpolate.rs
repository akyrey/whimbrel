//! `{{variables}}` (FR-2.4, FR-4): resolving a request against scopes, the dynamic `{{$…}}`
//! values, and masking the secrets a resolved request used.

use std::collections::BTreeMap;

use crate::{Body, FormField, Header, Request};

/// A variable's value. A secret is `None` when its name is declared but the keychain holds no
/// value for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Plain(String),
    Secret(Option<String>),
}

/// One layer of variables (an example, `.local`, an environment, the collection).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope(BTreeMap<String, Value>);

impl Scope {
    pub fn new(values: BTreeMap<String, Value>) -> Self {
        Self(values)
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }

    /// The names and values, in name order.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }
}

/// Where `{{$name}}` values come from; tests use fixed ones.
pub trait Dynamic {
    fn value(&self, name: &str) -> Option<String>;
}

/// `{{$uuid}}`, `{{$timestamp}}` (Unix seconds), `{{$isoTimestamp}}` (UTC), `{{$randomInt}}`
/// (0 to 1000).
pub struct SystemDynamic;

/// The names `SystemDynamic` answers, without the `$`.
pub const DYNAMIC_NAMES: [&str; 4] = ["uuid", "timestamp", "isoTimestamp", "randomInt"];

const RANDOM_INT_MAX: u32 = 1000;

impl Dynamic for SystemDynamic {
    fn value(&self, name: &str) -> Option<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        match name {
            "uuid" => Some(uuid::Uuid::new_v4().to_string()),
            "timestamp" => Some(now.to_string()),
            "isoTimestamp" => Some(iso_utc(now)),
            "randomInt" => {
                let bytes = uuid::Uuid::new_v4().into_bytes();
                let n = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Some((n % (RANDOM_INT_MAX + 1)).to_string())
            }
            _ => None,
        }
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` for Unix seconds (civil-from-days, Howard Hinnant).
fn iso_utc(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let rest = seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Why a request cannot be resolved: every unknown name and unset secret, each once.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}", describe(.undefined, .unset_secrets))]
pub struct ResolveError {
    pub undefined: Vec<String>,
    pub unset_secrets: Vec<String>,
}

fn describe(undefined: &[String], unset: &[String]) -> String {
    let mut parts = Vec::new();
    if !undefined.is_empty() {
        parts.push(format!("Not defined: {}", undefined.join(", ")));
    }
    if !unset.is_empty() {
        parts.push(format!(
            "No value in the keychain for: {}",
            unset.join(", ")
        ));
    }
    parts.join(". ")
}

/// A request with its variables replaced, and the secret values it now contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub request: Request,
    pub secrets: Vec<String>,
}

/// Text with its variables replaced, and what could not be.
#[derive(Debug, Default)]
pub struct Interpolated {
    pub text: String,
    pub secrets: Vec<String>,
    pub undefined: Vec<String>,
    pub unset_secrets: Vec<String>,
}

fn is_name(name: &str) -> bool {
    let body = name.strip_prefix('$').unwrap_or(name);
    let mut chars = body.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}

/// The variable names in `text`, in order (dynamic ones with their `$`).
fn names_in(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        let name = after[..end].trim();
        if is_name(name) {
            names.push(name.to_owned());
            rest = &after[end + 2..];
        } else {
            rest = &rest[start + 1..];
        }
    }
    names
}

fn push_once(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|x| x == item) {
        list.push(item.to_owned());
    }
}

fn lookup(
    name: &str,
    scopes: &[Scope],
    dynamic: &dyn Dynamic,
    out: &mut Interpolated,
) -> Option<String> {
    if let Some(dynamic_name) = name.strip_prefix('$') {
        let value = dynamic.value(dynamic_name);
        if value.is_none() {
            push_once(&mut out.undefined, name);
        }
        return value;
    }
    match scopes.iter().find_map(|s| s.get(name)) {
        Some(Value::Plain(value)) => Some(value.clone()),
        Some(Value::Secret(Some(value))) => {
            push_once(&mut out.secrets, value);
            Some(value.clone())
        }
        Some(Value::Secret(None)) => {
            push_once(&mut out.unset_secrets, name);
            None
        }
        None => {
            push_once(&mut out.undefined, name);
            None
        }
    }
}

/// Replaces every `{{name}}` in `text` it can; the rest is left as written and reported.
pub fn interpolate(text: &str, scopes: &[Scope], dynamic: &dyn Dynamic) -> Interpolated {
    let mut out = Interpolated::default();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        let name = after[..end].trim();
        if !is_name(name) {
            out.text.push_str(&rest[..=start]);
            rest = &rest[start + 1..];
            continue;
        }
        out.text.push_str(&rest[..start]);
        match lookup(name, scopes, dynamic, &mut out) {
            Some(value) => out.text.push_str(&value),
            None => out.text.push_str(&rest[start..start + 2 + end + 2]),
        }
        rest = &after[end + 2..];
    }
    out.text.push_str(rest);
    out
}

/// Every text of `request` that is sent: URL, enabled headers, body.
fn texts(request: &Request) -> Vec<&str> {
    let mut texts = vec![request.url.as_str()];
    for header in request.enabled_headers() {
        texts.push(&header.name);
        texts.push(&header.value);
    }
    match &request.body {
        Body::Text { text, .. } => texts.push(text),
        Body::File { path } => texts.push(path),
        body => {
            for field in body.enabled_fields() {
                texts.push(&field.name);
                texts.push(&field.value);
            }
        }
    }
    texts
}

/// The names `request` uses, each once, in order: for the "not defined" hints in the editor.
pub fn names_used(request: &Request) -> Vec<String> {
    let mut names = Vec::new();
    for text in texts(request) {
        for name in names_in(text) {
            push_once(&mut names, &name);
        }
    }
    names
}

/// Resolves every variable of `request` (scopes first to last), or says what is missing.
pub fn resolve(
    request: &Request,
    scopes: &[Scope],
    dynamic: &dyn Dynamic,
) -> Result<Resolved, ResolveError> {
    let mut all = Interpolated::default();
    let mut run = |text: &str| {
        let done = interpolate(text, scopes, dynamic);
        done.secrets
            .iter()
            .for_each(|s| push_once(&mut all.secrets, s));
        done.undefined
            .iter()
            .for_each(|s| push_once(&mut all.undefined, s));
        done.unset_secrets
            .iter()
            .for_each(|s| push_once(&mut all.unset_secrets, s));
        done.text
    };
    let headers = request
        .headers
        .iter()
        .map(|h| {
            if !h.enabled {
                return h.clone();
            }
            Header {
                name: run(&h.name),
                value: run(&h.value),
                enabled: true,
            }
        })
        .collect();
    let resolved = Request {
        method: request.method.clone(),
        url: run(&request.url),
        headers,
        body: match &request.body {
            Body::None => Body::None,
            Body::Text { kind, text } => Body::Text {
                kind: *kind,
                text: run(text),
            },
            Body::Form { fields } => Body::Form {
                fields: resolve_fields(fields, &mut run),
            },
            Body::Multipart { fields } => Body::Multipart {
                fields: resolve_fields(fields, &mut run),
            },
            Body::File { path } => Body::File { path: run(path) },
        },
    };
    if !all.undefined.is_empty() || !all.unset_secrets.is_empty() {
        return Err(ResolveError {
            undefined: all.undefined,
            unset_secrets: all.unset_secrets,
        });
    }
    Ok(Resolved {
        request: resolved,
        secrets: all.secrets,
    })
}

/// Enabled fields with their variables replaced; disabled ones as they are.
fn resolve_fields(fields: &[FormField], run: &mut impl FnMut(&str) -> String) -> Vec<FormField> {
    fields
        .iter()
        .map(|f| {
            if !f.enabled {
                return f.clone();
            }
            FormField {
                name: run(&f.name),
                value: run(&f.value),
                ..f.clone()
            }
        })
        .collect()
}

/// `request` with every secret value in `secrets` masked: URL, headers and every body kind.
pub fn mask_request(request: &Request, secrets: &[String]) -> Request {
    let m = |text: &str| mask(text, secrets);
    let fields = |fields: &[FormField]| -> Vec<FormField> {
        fields
            .iter()
            .map(|f| FormField {
                name: m(&f.name),
                value: m(&f.value),
                ..f.clone()
            })
            .collect()
    };
    Request {
        method: request.method.clone(),
        url: m(&request.url),
        headers: request
            .headers
            .iter()
            .map(|h| Header {
                name: m(&h.name),
                value: m(&h.value),
                enabled: h.enabled,
            })
            .collect(),
        body: match &request.body {
            Body::None => Body::None,
            Body::Text { kind, text } => Body::Text {
                kind: *kind,
                text: m(text),
            },
            Body::Form { fields: f } => Body::Form { fields: fields(f) },
            Body::Multipart { fields: f } => Body::Multipart { fields: fields(f) },
            Body::File { path } => Body::File { path: m(path) },
        },
    }
}

/// What a secret looks like where it must not be read.
pub const MASK: &str = "••••••";

/// `text` with every non-empty value of `secrets` replaced by [`MASK`].
pub fn mask(text: &str, secrets: &[String]) -> String {
    let mut sorted: Vec<&String> = secrets.iter().filter(|s| !s.is_empty()).collect();
    // Longer first, so a secret that contains another is masked whole.
    sorted.sort_by_key(|s| std::cmp::Reverse(s.len()));
    sorted.iter().fold(text.to_owned(), |acc, secret| {
        acc.replace(secret.as_str(), MASK)
    })
}

#[cfg(test)]
mod tests;
