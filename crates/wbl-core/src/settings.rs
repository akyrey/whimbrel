//! How a request is sent (FR-2.1): its own timeout, whether redirects are followed, whether the
//! server's certificate is checked.

use serde::{Deserialize, Serialize};

/// The defaults are what a request without `[settings]` gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RequestSettings {
    /// `None`: the app's default (30 s).
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub timeout_ms: Option<u64>,
    pub follow_redirects: bool,
    /// Turning this off accepts any certificate: for local servers with self-signed ones.
    pub verify_tls: bool,
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            follow_redirects: true,
            verify_tls: true,
        }
    }
}

impl RequestSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}
