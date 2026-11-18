//! What the commands share: one HTTP sender (and its connection pool) and the in-flight requests.

use wbl_http::{SendOptions, Sender};

use crate::error::AppError;
use crate::registry::InFlight;

pub struct AppState {
    pub sender: Sender,
    pub in_flight: InFlight,
    /// Where long bodies are written; emptied at start.
    spill_dir: Option<std::path::PathBuf>,
    /// Sent requests (FR-8); `None` in tests that do not need it.
    pub history: Option<wbl_store::History>,
    pub oauth: wbl_auth::OAuthClient,
}

impl AppState {
    pub fn new(options: SendOptions) -> Result<Self, AppError> {
        let spill_dir = options.spill_dir.clone();
        if let Some(dir) = &spill_dir {
            // Bodies of the last run are not needed: start empty (a missing folder is fine).
            let _ = std::fs::remove_dir_all(dir);
        }
        Ok(Self {
            sender: Sender::new(options)?,
            in_flight: InFlight::default(),
            spill_dir,
            history: None,
            oauth: wbl_auth::OAuthClient::new(),
        })
    }

    pub fn with_history(self, history: wbl_store::History) -> Self {
        Self {
            history: Some(history),
            ..self
        }
    }

    /// `file` if it is a body this app wrote; anything else is refused.
    pub fn spilled(&self, file: &str) -> Result<std::path::PathBuf, AppError> {
        let refuse = || AppError::new("invalid_path", "That is not a response body of this app");
        let dir = self
            .spill_dir
            .as_ref()
            .ok_or_else(refuse)?
            .canonicalize()
            .map_err(|_| refuse())?;
        let path = std::path::Path::new(file)
            .canonicalize()
            .map_err(|_| refuse())?;
        if path.starts_with(&dir) {
            Ok(path)
        } else {
            Err(refuse())
        }
    }
}
