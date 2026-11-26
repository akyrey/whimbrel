//! Security requirements → request auth, and the variables and secrets the auth refers to.

use wbl_core::auth::{Auth, Grant, KeyLocation, OAuth2Config};

use crate::model::{OAuthFlow, SchemeKind, SecurityRequirement, SecurityScheme, SecurityUse};

/// What an auth needs from the environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Needs {
    pub variables: Vec<(String, String)>,
    /// Name and description (where the value comes from).
    pub secrets: Vec<(String, String)>,
}

impl Needs {
    fn variable(&mut self, name: String, description: &str) -> String {
        if !self.variables.iter().any(|(n, _)| *n == name) {
            self.variables.push((name.clone(), description.to_owned()));
        }
        format!("{{{{{name}}}}}")
    }

    fn secret(&mut self, name: String, description: &str) -> String {
        if !self.secrets.iter().any(|(n, _)| *n == name) {
            self.secrets.push((name.clone(), description.to_owned()));
        }
        format!("{{{{{name}}}}}")
    }
}

/// `petstore_auth` → `petstoreAuth`.
pub fn identifier(name: &str) -> String {
    let mut out = String::new();
    for (i, word) in name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .enumerate()
    {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            if i == 0 {
                out.push(first.to_ascii_lowercase());
            } else {
                out.push(first.to_ascii_uppercase());
            }
            out.extend(chars);
        }
    }
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("auth{out}")
    } else {
        out
    }
}

const GRANTS: [(&str, Grant); 3] = [
    ("client_credentials", Grant::ClientCredentials),
    ("authorization_code", Grant::AuthorizationCode),
    ("device_code", Grant::DeviceCode),
];

fn oauth(
    id: &str,
    scheme: &str,
    flows: &[OAuthFlow],
    scopes: &[String],
    needs: &mut Needs,
) -> Option<Auth> {
    let (flow, grant) = GRANTS
        .iter()
        .find_map(|(kind, grant)| flows.iter().find(|f| f.kind == *kind).map(|f| (f, *grant)))?;
    let from = format!("OAuth 2 client of {scheme}");
    Some(Auth::OAuth2(OAuth2Config {
        grant,
        token_url: flow.token_url.clone().unwrap_or_default(),
        auth_url: flow.authorization_url.clone().unwrap_or_default(),
        device_url: flow.device_url.clone().unwrap_or_default(),
        client_id: needs.variable(format!("{id}ClientId"), &from),
        client_secret: needs.secret(format!("{id}ClientSecret"), &from),
        scope: scopes.join(" "),
        redirect_uri: String::new(),
    }))
}

/// The auth for one use of a scheme, or why there is none.
fn one(scheme: &SecurityScheme, used: &SecurityUse, needs: &mut Needs) -> Result<Auth, String> {
    let id = identifier(&scheme.name);
    let from = format!("For {}", scheme.name);
    match &scheme.kind {
        SchemeKind::Http { scheme: kind, .. } if kind == "basic" => Ok(Auth::Basic {
            username: needs.variable(format!("{id}Username"), &from),
            password: needs.secret(format!("{id}Password"), &from),
        }),
        SchemeKind::Http { scheme: kind, .. } if kind == "bearer" => Ok(Auth::Bearer {
            token: needs.secret(format!("{id}Token"), &from),
        }),
        SchemeKind::ApiKey { name, location } if location != "cookie" => Ok(Auth::ApiKey {
            name: name.clone(),
            value: needs.secret(id, &from),
            location: if location == "query" {
                KeyLocation::Query
            } else {
                KeyLocation::Header
            },
        }),
        SchemeKind::OAuth2 { flows, .. } => oauth(&id, &scheme.name, flows, &used.scopes, needs)
            .ok_or_else(|| {
                format!(
                    "{}: only implicit or password flows, which Whimbrel does not run",
                    scheme.name
                )
            }),
        other => Err(format!(
            "{}: {} auth is not supported",
            scheme.name,
            kind_name(other)
        )),
    }
}

fn kind_name(kind: &SchemeKind) -> String {
    match kind {
        SchemeKind::ApiKey { .. } => "cookie API key".into(),
        SchemeKind::Http { scheme, .. } => format!("HTTP {scheme}"),
        SchemeKind::OAuth2 { .. } => "OAuth 2".into(),
        SchemeKind::OpenIdConnect { .. } => "OpenID Connect".into(),
        SchemeKind::MutualTls => "mutual TLS (set a client certificate on the collection)".into(),
        SchemeKind::Unknown { kind } => kind.clone(),
    }
}

/// The auth satisfying the first requirement Whimbrel can meet; an empty requirement (auth is
/// optional) only when nothing else can be met.
pub fn auth_for(
    requirements: &[SecurityRequirement],
    schemes: &[SecurityScheme],
    needs: &mut Needs,
    warnings: &mut Vec<String>,
) -> Auth {
    if requirements.is_empty() {
        return Auth::None;
    }
    for requirement in requirements.iter().filter(|r| !r.uses.is_empty()) {
        for used in &requirement.uses {
            let Some(scheme) = schemes.iter().find(|s| s.name == used.scheme) else {
                warnings.push(format!("Unknown security scheme {}", used.scheme));
                continue;
            };
            match one(scheme, used, needs) {
                Ok(auth) => {
                    if requirement.uses.len() > 1 {
                        warnings.push(format!(
                            "Only {} of a combined requirement is applied",
                            scheme.name
                        ));
                    }
                    return auth;
                }
                Err(why) => warnings.push(why),
            }
        }
    }
    Auth::None
}
