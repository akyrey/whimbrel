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
