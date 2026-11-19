//! Documents → [`Api`]: the version, then the 3.x builder (2.0 is upconverted first).

use serde_json::Value;
use url::Url;

use crate::model::{
    Api, OAuthFlow, SchemeKind, SecurityRequirement, SecurityScheme, SecurityUse, Server,
    ServerVariable, SpecVersion, Tag,
};
use crate::read::Documents;
use crate::refs::Resolver;
use crate::{OpenApiError, operations, swagger2};

pub(crate) fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Text, or a number written as text (YAML reads `version: 1.0` as a number).
fn scalar(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub(crate) fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(crate) fn entries<'v>(
    value: &'v Value,
    key: &str,
) -> impl Iterator<Item = (&'v String, &'v Value)> {
    value
        .get(key)
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(k, _)| !k.starts_with("x-"))
}

/// The version a document declares; a newer 3.x is read as 3.2, with a warning.
pub fn detect(doc: &Value) -> Result<(SpecVersion, String, Option<String>), OpenApiError> {
    if doc.get("swagger").is_some() {
        let text = scalar(doc, "swagger").unwrap_or_default();
        return if text.starts_with("2.") {
            Ok((SpecVersion::Swagger2, text, None))
        } else {
            Err(OpenApiError::NotOpenApi(format!(
                "Swagger {text} is not supported (only 2.0)"
            )))
        };
    }
    let Some(text) = scalar(doc, "openapi") else {
        return Err(OpenApiError::NotOpenApi(
            "it has neither an `openapi` nor a `swagger` field".into(),
        ));
    };
    let mut parts = text.split('.').map(|p| p.parse::<u32>().ok());
    let version = match (parts.next().flatten(), parts.next().flatten()) {
        (Some(3), Some(0)) => (SpecVersion::OpenApi30, None),
        (Some(3), Some(1)) => (SpecVersion::OpenApi31, None),
        (Some(3), Some(2)) => (SpecVersion::OpenApi32, None),
        (Some(3), Some(_)) => (
            SpecVersion::OpenApi32,
            Some(format!("OpenAPI {text} is newer than 3.2; read as 3.2")),
        ),
        _ => {
            return Err(OpenApiError::NotOpenApi(format!(
                "OpenAPI {text} is not supported"
            )));
        }
    };
    Ok((version.0, text, version.1))
}

/// Reads a loaded spec into the model.
pub fn build(documents: Documents) -> Result<Api, OpenApiError> {
    let entry = documents.entry.clone().unwrap_or_else(placeholder_url);
    let doc = documents
        .entry_value()
        .ok_or_else(|| OpenApiError::Load("The entry document is missing".into()))?;
    let (version, version_text, warning) = detect(doc)?;
    let documents = if version == SpecVersion::Swagger2 {
        upconverted(documents, &entry)
    } else {
        documents
    };
    let mut resolver = Resolver::new(&documents);
    resolver.legacy_schemas = matches!(version, SpecVersion::Swagger2 | SpecVersion::OpenApi30);
    resolver.warnings.extend(warning);
    let doc = documents.entry_value().unwrap_or(&Value::Null);
    Ok(read_v3(&mut resolver, &entry, doc, version, version_text))
}

fn placeholder_url() -> Url {
    // A fixed, valid URL: only joins relative references against it.
    Url::parse("file:///openapi.json").unwrap_or_else(|_| unreachable!())
}

fn upconverted(documents: Documents, entry: &Url) -> Documents {
    let by_url = documents
        .by_url
        .iter()
        .map(|(url, value)| {
            let is_entry = documents
                .entry
                .as_ref()
                .is_some_and(|e| documents.get(e).is_some_and(|v| std::ptr::eq(v, value)));
            let converted = if is_entry {
                swagger2::upconvert(value, Some(entry))
            } else {
                swagger2::rewrite_refs(value)
            };
            (url.clone(), converted)
        })
        .collect();
    Documents {
        entry: documents.entry,
        by_url,
    }
}

fn read_v3<'a>(
    resolver: &mut Resolver<'a>,
    entry: &Url,
    doc: &'a Value,
    version: SpecVersion,
    version_text: String,
) -> Api {
    let info = doc.get("info").unwrap_or(&Value::Null);
    for (name, _) in entries(doc.get("components").unwrap_or(&Value::Null), "schemas") {
        if let Ok(url) = entry.join(&format!(
            "#/components/schemas/{}",
            name.replace('~', "~0").replace('/', "~1")
        )) {
            resolver.register(&url);
        }
    }
    let operations = operations::paths(resolver, entry, doc.get("paths"));
    let webhooks = operations::paths(resolver, entry, doc.get("webhooks"));
    let security_schemes = security_schemes(resolver, entry, doc);
    resolver.finish();
    Api {
        version,
        version_text,
        title: text(info, "title").unwrap_or_else(|| "API".into()),
        api_version: scalar(info, "version").unwrap_or_default(),
        description: text(info, "description"),
        servers: servers(entry, doc.get("servers")),
        tags: tags(doc),
        operations,
        webhooks,
        security_schemes,
        security: requirements(doc.get("security")).unwrap_or_default(),
        schemas: std::mem::take(&mut resolver.schemas),
        warnings: std::mem::take(&mut resolver.warnings),
    }
}

/// Servers with relative URLs resolved against the document's own URL when it was fetched.
pub(crate) fn servers(entry: &Url, value: Option<&Value>) -> Vec<Server> {
    let remote = matches!(entry.scheme(), "http" | "https");
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let url = text(s, "url")?;
            let url = if remote && !url.contains("://") && !url.starts_with('{') {
                entry
                    .join(&url)
                    .map_or(url, |u| u.to_string().trim_end_matches('/').to_owned())
            } else {
                url
            };
            let variables = entries(s, "variables")
                .map(|(name, v)| ServerVariable {
                    name: name.clone(),
                    default: scalar(v, "default").unwrap_or_default(),
                    values: v
                        .get("enum")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                    description: text(v, "description"),
                })
                .collect();
            Some(Server {
                url,
                description: text(s, "description"),
                variables,
            })
        })
        .collect()
}

fn tags(doc: &Value) -> Vec<Tag> {
    doc.get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|t| {
            Some(Tag {
                name: text(t, "name")?,
                summary: text(t, "summary"),
                description: text(t, "description"),
                parent: text(t, "parent"),
                kind: text(t, "kind"),
            })
        })
        .collect()
}

pub(crate) fn requirements(value: Option<&Value>) -> Option<Vec<SecurityRequirement>> {
    let items = value?.as_array()?;
    Some(
        items
            .iter()
            .filter_map(Value::as_object)
            .map(|req| SecurityRequirement {
                uses: req
                    .iter()
                    .map(|(scheme, scopes)| SecurityUse {
                        scheme: scheme.clone(),
                        scopes: scopes
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    )
}

const FLOWS: [(&str, &str); 5] = [
    ("clientCredentials", "client_credentials"),
    ("authorizationCode", "authorization_code"),
    ("deviceAuthorization", "device_code"),
    ("password", "password"),
    ("implicit", "implicit"),
];

fn oauth_flows(value: &Value) -> Vec<OAuthFlow> {
    let flows = value.get("flows").unwrap_or(&Value::Null);
    FLOWS
        .iter()
        .filter_map(|(key, kind)| {
            let f = flows.get(*key)?;
            Some(OAuthFlow {
                kind: (*kind).to_owned(),
                authorization_url: text(f, "authorizationUrl"),
                token_url: text(f, "tokenUrl"),
                device_url: text(f, "deviceAuthorizationUrl"),
                scopes: f
                    .get("scopes")
                    .and_then(Value::as_object)
                    .map(|s| s.keys().cloned().collect())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn scheme_kind(value: &Value) -> SchemeKind {
    match text(value, "type").unwrap_or_default().as_str() {
        "apiKey" => SchemeKind::ApiKey {
            name: text(value, "name").unwrap_or_default(),
            location: text(value, "in").unwrap_or_else(|| "header".into()),
        },
        "http" => SchemeKind::Http {
            scheme: text(value, "scheme")
                .unwrap_or_default()
                .to_ascii_lowercase(),
            bearer_format: text(value, "bearerFormat"),
        },
        "oauth2" => SchemeKind::OAuth2 {
            flows: oauth_flows(value),
            metadata_url: text(value, "oauth2MetadataUrl"),
        },
        "openIdConnect" => SchemeKind::OpenIdConnect {
            url: text(value, "openIdConnectUrl").unwrap_or_default(),
        },
        "mutualTLS" => SchemeKind::MutualTls,
        other => SchemeKind::Unknown {
            kind: other.to_owned(),
        },
    }
}

fn security_schemes<'a>(
    resolver: &mut Resolver<'a>,
    entry: &Url,
    doc: &'a Value,
) -> Vec<SecurityScheme> {
    let Some(components) = doc.get("components") else {
        return Vec::new();
    };
    entries(components, "securitySchemes")
        .map(|(name, value)| {
            let (_, value) = resolver.deref(entry, value);
            SecurityScheme {
                name: name.clone(),
                description: text(value, "description"),
                deprecated: flag(value, "deprecated"),
                kind: scheme_kind(value),
            }
        })
        .collect()
}
