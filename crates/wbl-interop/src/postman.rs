//! Postman v2.0/v2.1 collections and environments. Secrets become secret names, never values;
//! scripts are not run, and say so.

use std::collections::BTreeMap;

use serde_json::Value;
use wbl_core::auth::{Auth, Grant, KeyLocation, OAuth2Config, loopback_redirect};
use wbl_core::example::overlay_of;
use wbl_core::{Body, BodyKind, FormField, Header, Request};
use wbl_format::{Environment, SecretName};

use crate::{Imported, ImportedRequest, InteropError, folders_of, stored};

/// Postman's dynamic variables Whimbrel has, by Postman's name.
const DYNAMIC: [(&str, &str); 4] = [
    ("{{$guid}}", "{{$uuid}}"),
    ("{{$randomUUID}}", "{{$uuid}}"),
    ("{{$isoTimestamp}}", "{{$isoTimestamp}}"),
    ("{{$timestamp}}", "{{$timestamp}}"),
];

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn list<'v>(value: &'v Value, key: &str) -> impl Iterator<Item = &'v Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn disabled(value: &Value) -> bool {
    value
        .get("disabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

struct Reader {
    warnings: Vec<String>,
}

impl Reader {
    fn warn(&mut self, warning: String) {
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }

    /// Postman's dynamic variables mapped to ours; the others are warned about.
    fn vars(&mut self, text: &str) -> String {
        let mut out = text.to_owned();
        for (theirs, ours) in DYNAMIC {
            out = out.replace(theirs, ours);
        }
        if let Some(at) = out.find("{{$") {
            let name: String = out[at + 2..].chars().take_while(|c| *c != '}').collect();
            if !["$uuid", "$isoTimestamp", "$timestamp", "$randomInt"].contains(&name.as_str()) {
                self.warn(format!(
                    "Postman's {name} has no equivalent; it is left as it is"
                ));
            }
        }
        out
    }

    fn url(&mut self, url: Option<&Value>) -> String {
        let raw = match url {
            Some(Value::String(s)) => s.clone(),
            Some(u) => u
                .get("raw")
                .and_then(Value::as_str)
                .map_or_else(|| rebuild_url(u), str::to_owned),
            None => String::new(),
        };
        self.vars(&raw)
    }

    fn fields(&mut self, items: &Value, key: &str, files: bool) -> Vec<FormField> {
        let fields: Vec<FormField> = list(items, key)
            .map(|f| {
                let file = files && text(f, "type") == "file";
                let value = if file { src_of(f) } else { text(f, "value") };
                FormField {
                    name: text(f, "key"),
                    value,
                    enabled: !disabled(f),
                    file,
                }
            })
            .collect();
        fields
            .into_iter()
            .map(|f| FormField {
                name: self.vars(&f.name),
                value: self.vars(&f.value),
                ..f
            })
            .collect()
    }

    fn body(&mut self, body: Option<&Value>) -> Body {
        let Some(body) = body else { return Body::None };
        match text(body, "mode").as_str() {
            "raw" => {
                let language = body
                    .pointer("/options/raw/language")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let kind = match language {
                    "json" => BodyKind::Json,
                    "xml" => BodyKind::Xml,
                    _ => BodyKind::Text,
                };
                Body::Text {
                    kind,
                    text: self.vars(&text(body, "raw")),
                }
            }
            "urlencoded" => Body::Form {
                fields: self.fields(body, "urlencoded", false),
            },
            "formdata" => Body::Multipart {
                fields: self.fields(body, "formdata", true),
            },
            "file" => Body::File {
                path: body
                    .pointer("/file/src")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            },
            "graphql" => {
                let query = body
                    .pointer("/graphql/query")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let variables = body
                    .pointer("/graphql/variables")
                    .and_then(Value::as_str)
                    .unwrap_or("{}");
                let vars: Value =
                    serde_json::from_str(variables).unwrap_or(Value::Object(Default::default()));
                let json = serde_json::json!({ "query": query, "variables": vars });
                Body::Text {
                    kind: BodyKind::Json,
                    text: serde_json::to_string_pretty(&json).unwrap_or_default(),
                }
            }
            "" => Body::None,
            other => {
                self.warn(format!("A {other} body was not imported"));
                Body::None
            }
        }
    }

    fn request(&mut self, r: &Value) -> Request {
        let headers = list(r, "header")
            .map(|h| Header {
                name: text(h, "key"),
                value: text(h, "value"),
                enabled: !disabled(h),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| Header {
                value: self.vars(&h.value),
                ..h
            })
            .collect();
        Request {
            method: r
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("GET")
                .to_ascii_uppercase(),
            url: self.url(r.get("url")),
            headers,
            body: self.body(r.get("body")),
        }
    }

    /// Postman's callback URL when Whimbrel can listen on it (or it is a variable); otherwise
    /// empty, for a random loopback port, with a warning.
    fn redirect_uri(&mut self, uri: String) -> String {
        if uri.is_empty() || uri.contains("{{") || loopback_redirect(&uri).is_ok() {
            return uri;
        }
        self.warn(format!(
            "the OAuth 2 callback URL {uri} is not a loopback address; Whimbrel uses a random \
             port on 127.0.0.1 instead, which the provider must accept"
        ));
        String::new()
    }

    /// Postman auth; `None` (absent) inherits.
    fn auth(&mut self, auth: Option<&Value>, secrets: &mut Vec<String>) -> Auth {
        let Some(auth) = auth else {
            return Auth::Inherit;
        };
        let kind = text(auth, "type");
        let field = |name: &str| -> String {
            list(auth, &kind)
                .find(|f| text(f, "key") == name)
                .map(|f| match f.get("value") {
                    Some(Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                    None => String::new(),
                })
                .unwrap_or_default()
        };
        // Credentials typed into Postman become secret names; references stay references.
        let mut secret = |value: String, name: &str| -> String {
            if value.is_empty() || value.contains("{{") {
                return value;
            }
            secrets.push(name.to_owned());
            format!("{{{{{name}}}}}")
        };
        match kind.as_str() {
            "noauth" => Auth::None,
            "bearer" => Auth::Bearer {
                token: secret(field("token"), "bearerToken"),
            },
            "basic" => Auth::Basic {
                username: field("username"),
                password: secret(field("password"), "basicPassword"),
            },
            "apikey" => Auth::ApiKey {
                name: field("key"),
                value: secret(field("value"), "apiKey"),
                location: if field("in") == "query" {
                    KeyLocation::Query
                } else {
                    KeyLocation::Header
                },
            },
            "oauth2" => {
                let grant = match field("grant_type").as_str() {
                    "authorization_code" | "authorization_code_with_pkce" => {
                        Grant::AuthorizationCode
                    }
                    _ => Grant::ClientCredentials,
                };
                Auth::OAuth2(OAuth2Config {
                    grant,
                    token_url: field("accessTokenUrl"),
                    auth_url: field("authUrl"),
                    device_url: String::new(),
                    client_id: field("clientId"),
                    client_secret: secret(field("clientSecret"), "clientSecret"),
                    scope: field("scope"),
                    redirect_uri: self.redirect_uri(field("redirect_uri")),
                })
            }
            other => {
                self.warn(format!(
                    "{other} auth is not supported; those requests inherit"
                ));
                Auth::Inherit
            }
        }
    }

    fn walk(
        &mut self,
        items: &Value,
        folder: &[String],
        out: &mut Vec<ImportedRequest>,
        folders: &mut Vec<Vec<String>>,
        secrets: &mut Vec<String>,
    ) {
        for item in list(items, "item") {
            let name = text(item, "name");
            if !list(item, "event").collect::<Vec<_>>().is_empty() {
                self.warn("Pre-request and test scripts are not run in Whimbrel; use captures and assertions".into());
            }
            if item.get("item").is_some() {
                let mut chain = folder.to_vec();
                chain.push(name);
                folders.push(chain.clone());
                self.walk(item, &chain, out, folders, secrets);
                continue;
            }
            let Some(r) = item.get("request") else {
                continue;
            };
            let request = if let Value::String(url) = r {
                Request {
                    method: "GET".into(),
                    url: self.vars(url),
                    headers: Vec::new(),
                    body: Body::None,
                }
            } else {
                self.request(r)
            };
            let mut stored = stored(&name, request);
            stored.auth = self.auth(r.get("auth"), secrets);
            stored.examples = self.examples(item, &stored.request);
            out.push(ImportedRequest {
                folder: folder.to_vec(),
                stored,
            });
        }
    }

    /// Saved responses whose request differs only in query, headers or body become examples.
    fn examples(&mut self, item: &Value, base: &Request) -> Vec<wbl_core::example::Example> {
        let mut out = Vec::new();
        for (i, response) in list(item, "response").enumerate() {
            let Some(original) = response.get("originalRequest") else {
                continue;
            };
            let name = text(response, "name");
            let draft = self.request(original);
            let id = format!("postman-{}", i + 1);
            match overlay_of(base, &draft, &id, if name.is_empty() { &id } else { &name }) {
                Ok(example) => out.push(wbl_core::example::Example {
                    order: i64::try_from(i + 1).unwrap_or(0) * 10,
                    ..example
                }),
                Err(_) => self.warn(format!(
                    "The saved response {name} changes the method or path; it was not imported"
                )),
            }
        }
        out
    }
}

fn src_of(field: &Value) -> String {
    match field.get("src") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .first()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    }
}

/// A URL from Postman's parts when it has no `raw`.
fn rebuild_url(u: &Value) -> String {
    let join = |key: &str, sep: &str| -> String {
        match u.get(key) {
            Some(Value::Array(parts)) => parts
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(sep),
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        }
    };
    let protocol = u
        .get("protocol")
        .and_then(Value::as_str)
        .map(|p| format!("{p}://"))
        .unwrap_or_default();
    let query: Vec<String> = list(u, "query")
        .filter(|q| !disabled(q))
        .map(|q| format!("{}={}", text(q, "key"), text(q, "value")))
        .collect();
    let mut url = format!("{protocol}{}/{}", join("host", "."), join("path", "/"));
    if !query.is_empty() {
        url = format!("{url}?{}", query.join("&"));
    }
    url
}

/// A Postman environment: plain values as variables, secret ones as names only.
fn environment(doc: &Value) -> Environment {
    let mut variables = BTreeMap::new();
    let mut secrets = Vec::new();
    for v in
        list(doc, "values").filter(|v| v.get("enabled").and_then(Value::as_bool).unwrap_or(true))
    {
        if text(v, "type") == "secret" {
            secrets.push(SecretName {
                name: text(v, "key"),
                description: "From the Postman environment".into(),
            });
        } else {
            variables.insert(text(v, "key"), text(v, "value"));
        }
    }
    Environment {
        name: text(doc, "name"),
        protected: false,
        variables,
        secrets,
    }
}

fn is_environment(doc: &Value) -> bool {
    doc.get("values").is_some() && doc.get("item").is_none()
}

/// Reads a Postman collection (v2.0 or v2.1) or environment.
pub fn read(json: &str) -> Result<Imported, InteropError> {
    let doc: Value =
        serde_json::from_str(json).map_err(|e| InteropError::NotJson(e.to_string()))?;
    if is_environment(&doc) {
        let env = environment(&doc);
        return Ok(Imported {
            name: env.name.clone(),
            auth: Auth::Inherit,
            variables: BTreeMap::new(),
            folders: Vec::new(),
            requests: Vec::new(),
            environments: vec![env],
            warnings: Vec::new(),
        });
    }
    let info = doc.get("info").ok_or(InteropError::NotPostman)?;
    if !text(info, "schema").contains("collection/v2") {
        return Err(InteropError::NotPostman);
    }
    let mut reader = Reader {
        warnings: Vec::new(),
    };
    let mut requests = Vec::new();
    let mut folders = Vec::new();
    let mut secrets = Vec::new();
    reader.walk(&doc, &[], &mut requests, &mut folders, &mut secrets);
    if requests.is_empty() {
        return Err(InteropError::Empty);
    }
    let auth = match reader.auth(doc.get("auth"), &mut secrets) {
        Auth::Inherit => Auth::None,
        other => other,
    };
    let mut variables = BTreeMap::new();
    for v in list(&doc, "variable") {
        if text(v, "type") == "secret" {
            secrets.push(text(v, "key"));
        } else {
            variables.insert(text(v, "key"), text(v, "value"));
        }
    }
    secrets.sort();
    secrets.dedup();
    let environments = if secrets.is_empty() {
        Vec::new()
    } else {
        reader.warn("Credentials were not copied: set them in the Imported environment".into());
        vec![Environment {
            name: "Imported".into(),
            protected: false,
            variables: BTreeMap::new(),
            secrets: secrets
                .into_iter()
                .map(|name| SecretName {
                    name,
                    description: "From the Postman collection".into(),
                })
                .collect(),
        }]
    };
    Ok(Imported {
        name: text(info, "name"),
        auth,
        variables,
        folders: folders_of(&requests, &folders),
        requests,
        environments,
        warnings: reader.warnings,
    })
}
