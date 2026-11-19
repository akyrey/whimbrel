//! The collection an [`Api`] becomes (task 6.6; OPENAPI.md "Generation rules"). Pure: the
//! result is written by [`crate::install`].

mod auth;
mod request;
mod values;

use wbl_core::auth::Auth;
use wbl_format::{Environment, SecretName, StoredRequest};

pub use auth::identifier;
use auth::{Needs, auth_for};

use crate::model::{Api, Server, Tag};

/// Tag kinds that label operations rather than group them (OAS 3.2 registry).
const NON_GROUPING_KINDS: [&str; 2] = ["badge", "audience"];
const DEFAULT_BASE_URL: &str = "http://localhost";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GeneratedRequest {
    /// Folder names from the top.
    pub folder: Vec<String>,
    pub operation_id: String,
    pub stored: StoredRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Generated {
    pub name: String,
    /// The collection's auth (the API's default security).
    pub auth: Auth,
    /// Every folder, parents before children, as names from the top.
    pub folders: Vec<Vec<String>>,
    pub requests: Vec<GeneratedRequest>,
    pub environments: Vec<Environment>,
    pub warnings: Vec<String>,
}

fn tag_label(tag: &Tag) -> String {
    tag.summary
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| tag.name.clone())
}

/// The folder chain of a tag: its parents (3.2 `parent`) outermost first.
fn tag_folders(api: &Api, name: &str) -> Vec<String> {
    let mut chain = Vec::new();
    let mut at = Some(name.to_owned());
    while let Some(current) = at {
        if chain.len() > api.tags.len() {
            break; // a parent cycle
        }
        let tag = api.tags.iter().find(|t| t.name == current);
        chain.push(tag.map_or_else(|| current.clone(), tag_label));
        at = tag.and_then(|t| t.parent.clone());
    }
    chain.reverse();
    chain
}

fn grouping(api: &Api, tag: &str) -> bool {
    api.tags
        .iter()
        .find(|t| t.name == tag)
        .and_then(|t| t.kind.as_deref())
        .is_none_or(|k| !NON_GROUPING_KINDS.contains(&k))
}

/// The folder an operation goes in: its first grouping tag, else its first path segment.
fn folder_of(api: &Api, op: &crate::model::Operation) -> Vec<String> {
    if let Some(tag) = op.tags.iter().find(|t| grouping(api, t)) {
        return tag_folders(api, tag);
    }
    op.path
        .split('/')
        .find(|s| !s.is_empty() && !s.starts_with('{'))
        .map(|s| vec![s.to_owned()])
        .unwrap_or_default()
}

fn folders_of(requests: &[GeneratedRequest]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for request in requests {
        for depth in 1..=request.folder.len() {
            let prefix = request.folder[..depth].to_vec();
            if !out.contains(&prefix) {
                out.push(prefix);
            }
        }
    }
    out
}

fn environment_name(server: &Server, index: usize, taken: &[Environment]) -> String {
    let base = server
        .description
        .clone()
        .filter(|d| !d.trim().is_empty())
        .or_else(|| {
            url::Url::parse(&server.url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_owned))
        })
        .unwrap_or_else(|| format!("Server {}", index + 1));
    let mut name = base.clone();
    let mut n = 2;
    while taken.iter().any(|e| e.name == name) {
        name = format!("{base} {n}");
        n += 1;
    }
    name
}

fn environment(server: &Server, name: String, needs: &Needs) -> Environment {
    let base_url = server
        .url
        .replace('{', "{{")
        .replace('}', "}}")
        .trim_end_matches('/')
        .to_owned();
    let mut variables: std::collections::BTreeMap<String, String> = server
        .variables
        .iter()
        .map(|v| (v.name.clone(), v.default.clone()))
        .collect();
    variables.insert(
        "baseUrl".into(),
        if base_url.is_empty() {
            DEFAULT_BASE_URL.into()
        } else {
            base_url
        },
    );
    for (name, _) in &needs.variables {
        variables.entry(name.clone()).or_default();
    }
    Environment {
        name,
        protected: false,
        variables,
        secrets: needs
            .secrets
            .iter()
            .map(|(name, description)| SecretName {
                name: name.clone(),
                description: description.clone(),
            })
            .collect(),
    }
}

fn environments(api: &Api, needs: &Needs) -> Vec<Environment> {
    let servers = if api.servers.is_empty() {
        vec![Server {
            url: String::new(),
            description: Some("Default".into()),
            variables: Vec::new(),
        }]
    } else {
        api.servers.clone()
    };
    let mut out: Vec<Environment> = Vec::new();
    for (i, server) in servers.iter().enumerate() {
        let name = environment_name(server, i, &out);
        out.push(environment(server, name, needs));
    }
    out
}

/// The collection for `api`.
pub fn generate(api: &Api) -> Generated {
    let mut warnings = Vec::new();
    let mut needs = Needs::default();
    let auth = auth_for(
        &api.security,
        &api.security_schemes,
        &mut needs,
        &mut warnings,
    );
    let requests: Vec<GeneratedRequest> = api
        .operations
        .iter()
        .map(|op| {
            let mut stored = request::request(api, op, &mut warnings);
            stored.auth = match &op.security {
                Some(own) if *own != api.security => {
                    auth_for(own, &api.security_schemes, &mut needs, &mut warnings)
                }
                _ => Auth::Inherit,
            };
            GeneratedRequest {
                folder: folder_of(api, op),
                operation_id: op.id.clone(),
                stored,
            }
        })
        .collect();
    let mut all_warnings = api.warnings.clone();
    for w in warnings {
        if !all_warnings.contains(&w) {
            all_warnings.push(w);
        }
    }
    Generated {
        name: api.title.clone(),
        auth,
        folders: folders_of(&requests),
        environments: environments(api, &needs),
        requests,
        warnings: all_warnings,
    }
}
