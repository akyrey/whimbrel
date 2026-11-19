//! Collections in and out of other tools (D-H19): Postman v2.1 collections and environments, and
//! `.http` files (JetBrains, VS Code REST Client), into the same shape a Whimbrel collection is
//! written from.

pub mod http_export;
pub mod http_file;
pub mod install;
pub mod postman;

use wbl_core::auth::Auth;
use wbl_format::{Environment, StoredRequest};

pub use install::{Installed, install};

/// What an import makes, before it is written.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Imported {
    pub name: String,
    /// The collection's (or the import folder's) auth.
    pub auth: Auth,
    /// Collection variables.
    pub variables: std::collections::BTreeMap<String, String>,
    /// Every folder, parents before children, as names from the top.
    pub folders: Vec<Vec<String>>,
    pub requests: Vec<ImportedRequest>,
    pub environments: Vec<Environment>,
    /// What was not carried over (scripts, unsupported auth, …).
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ImportedRequest {
    pub folder: Vec<String>,
    pub stored: StoredRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InteropError {
    #[error("The file is not JSON: {0}")]
    NotJson(String),
    #[error("This is not a Postman v2 collection or environment")]
    NotPostman,
    #[error("No requests were found in the file")]
    Empty,
}

impl InteropError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotJson(_) => "interop_syntax",
            Self::NotPostman => "interop_unknown",
            Self::Empty => "interop_empty",
        }
    }
}

/// Reads a file of any supported kind: Postman JSON (collection or environment) or `.http`.
pub fn read(name: &str, text: &str) -> Result<Imported, InteropError> {
    if text.trim_start().starts_with('{') {
        postman::read(text)
    } else {
        http_file::read(name, text)
    }
}

/// A request with nothing set but its name and request.
pub(crate) fn stored(name: &str, request: wbl_core::Request) -> StoredRequest {
    StoredRequest {
        name: name.to_owned(),
        order: 0,
        request,
        examples: Vec::new(),
        settings: wbl_core::RequestSettings::default(),
        auth: Auth::Inherit,
        variables: Default::default(),
        captures: Vec::new(),
        assertions: Vec::new(),
    }
}

/// The folder chains a list of requests needs, parents first.
pub(crate) fn folders_of(requests: &[ImportedRequest], extra: &[Vec<String>]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let chains = requests
        .iter()
        .map(|r| r.folder.clone())
        .chain(extra.iter().cloned());
    for chain in chains {
        for depth in 1..=chain.len() {
            let prefix = chain[..depth].to_vec();
            if !out.contains(&prefix) {
                out.push(prefix);
            }
        }
    }
    out
}
