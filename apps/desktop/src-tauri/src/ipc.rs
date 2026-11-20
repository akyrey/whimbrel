//! Command arguments. Every command takes one `args` object (as in Dunlin).

use serde::Deserialize;
use ts_rs::TS;
use wbl_core::Request;

/// What a send takes; shared with the engine.
pub use wbl_engine::RequestSendArgs;

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RequestCancelArgs {
    pub request_id: String,
}

/// A collection, by the absolute path of its folder (as the dialog returned it).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CollectionArgs {
    pub root: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CollectionCreateArgs {
    pub root: String,
    pub name: String,
}

/// A request or folder in a collection; `path` is relative, with `/`.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NodeArgs {
    pub root: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RequestSaveArgs {
    pub root: String,
    pub path: String,
    pub stored: wbl_format::StoredRequest,
    /// The hash `request_read` (or the last save) returned.
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RequestCreateArgs {
    pub root: String,
    /// `""` for the collection's top level.
    pub folder: String,
    pub stored: wbl_format::StoredRequest,
}

/// New folder (`path` is the parent) or rename (`path` is the request).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NodeNameArgs {
    pub root: String,
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NodeMoveArgs {
    pub root: String,
    pub path: String,
    pub folder: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NodeOrderArgs {
    pub root: String,
    pub path: String,
    #[ts(type = "number")]
    pub order: i64,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentArgs {
    pub root: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCreateArgs {
    pub root: String,
    pub environment: wbl_format::Environment,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSaveArgs {
    pub root: String,
    pub id: String,
    pub environment: wbl_format::Environment,
    pub hash: String,
}

/// The user's own values (`.local`), or the collection's `[variables]` (`id` absent).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VariablesSaveArgs {
    pub root: String,
    #[ts(optional)]
    pub id: Option<String>,
    pub variables: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SecretSetArgs {
    pub root: String,
    pub environment: String,
    pub name: String,
    /// Goes to the keychain; never stored elsewhere, logged or returned.
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SecretArgs {
    pub root: String,
    pub environment: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExampleApplyArgs {
    pub request: Request,
    pub example: wbl_core::example::Example,
}

/// The example that turns `base` into `draft`, named `name`.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExampleOverlayArgs {
    pub base: Request,
    pub draft: Request,
    pub id: String,
    pub name: String,
}

/// Writes a response body to `destination` (chosen in the save dialog): a body that went to a
/// file is copied from it; a shown one is written from its text or base64.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResponseSaveArgs {
    pub destination: String,
    #[ts(optional)]
    pub file: Option<String>,
    #[ts(optional)]
    pub text: Option<String>,
    #[ts(optional)]
    pub base64: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryListArgs {
    pub query: String,
    #[ts(type = "number")]
    pub limit: i64,
    #[ts(type = "number")]
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryIdArgs {
    #[ts(type = "number")]
    pub id: i64,
}

/// The request as a cURL command; secrets masked unless `reveal`.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CurlExportArgs {
    pub send: RequestSendArgs,
    pub reveal: bool,
}

/// A JSONPath to run over a response body (task 9.4).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResponseSelectArgs {
    pub body: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CurlImportArgs {
    pub text: String,
}

/// A pasted cURL command as a request, with the options that were not used.
#[derive(Debug, Clone, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CurlImported {
    pub request: Request,
    pub settings: wbl_core::RequestSettings,
    pub ignored: Vec<String>,
}

/// The auth of a folder (`path`), or of the collection (`path` empty).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuthSaveArgs {
    pub root: String,
    pub path: String,
    pub auth: wbl_core::auth::Auth,
}

/// The headers a folder (`path`), or the collection (`path` empty), adds to its requests.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HeadersSaveArgs {
    pub root: String,
    pub path: String,
    pub headers: Vec<wbl_core::Header>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TlsSaveArgs {
    pub root: String,
    pub tls: wbl_format::CollectionTls,
}
