//! Command arguments. Every command takes one `args` object (as in Dunlin).

use serde::Deserialize;
use ts_rs::TS;
use wbl_core::Request;

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RequestSendArgs {
    /// The tab sending it; `request_cancel` names the same id.
    pub request_id: String,
    pub request: Request,
    /// The collection whose variables apply, and its active environment.
    #[ts(optional)]
    pub root: Option<String>,
    #[ts(optional)]
    pub environment: Option<String>,
    /// The example laid over the request, if one is picked.
    #[ts(optional)]
    pub example: Option<wbl_core::example::Example>,
}

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
pub struct RequestMoveArgs {
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
