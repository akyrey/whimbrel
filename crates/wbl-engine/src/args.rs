//! What a send takes. The app's IPC uses it as the `request_send` argument.

use serde::Deserialize;
use wbl_core::Request;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RequestSendArgs {
    /// The tab sending it; `request_cancel` names the same id.
    pub request_id: String,
    pub request: Request,
    /// The collection whose variables apply, and its active environment.
    #[cfg_attr(feature = "ts", ts(optional))]
    pub root: Option<String>,
    #[cfg_attr(feature = "ts", ts(optional))]
    pub environment: Option<String>,
    /// The example laid over the request, if one is picked.
    #[cfg_attr(feature = "ts", ts(optional))]
    pub example: Option<wbl_core::example::Example>,
    /// The request's own settings; the defaults when absent.
    #[cfg_attr(feature = "ts", ts(optional))]
    pub settings: Option<wbl_core::RequestSettings>,
    /// The request's name, for the history.
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// The request's file in the collection: its folders' auth is inherited.
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// The request's own auth (inherit when absent).
    #[cfg_attr(feature = "ts", ts(optional))]
    pub auth: Option<wbl_core::auth::Auth>,
    /// The request's `[variables]`: after the example, before the environment.
    #[cfg_attr(feature = "ts", ts(optional))]
    pub variables: Option<std::collections::BTreeMap<String, String>>,
    /// Values to take from the response into runtime variables.
    #[cfg_attr(feature = "ts", ts(optional))]
    pub captures: Option<Vec<wbl_core::check::Capture>>,
    /// Checks of the response.
    #[cfg_attr(feature = "ts", ts(optional))]
    pub assertions: Option<Vec<wbl_core::check::Assertion>>,
}

impl RequestSendArgs {
    /// The arguments to send `stored` (a request file at `path` in the collection at `root`).
    pub fn stored(
        request_id: &str,
        root: &str,
        path: &str,
        stored: &wbl_format::StoredRequest,
    ) -> Self {
        Self {
            request_id: request_id.to_owned(),
            request: stored.request.clone(),
            root: Some(root.to_owned()),
            environment: None,
            example: None,
            settings: Some(stored.settings),
            name: Some(stored.name.clone()),
            path: Some(path.to_owned()),
            auth: Some(stored.auth.clone()),
            variables: Some(stored.variables.clone()),
            captures: Some(stored.captures.clone()),
            assertions: Some(stored.assertions.clone()),
        }
    }
}
