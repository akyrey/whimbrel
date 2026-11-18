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
