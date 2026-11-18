//! What the commands share: one HTTP sender (and its connection pool) and the in-flight requests.

use wbl_http::{SendOptions, Sender};

use crate::error::AppError;
use crate::registry::InFlight;

pub struct AppState {
    pub sender: Sender,
    pub in_flight: InFlight,
}

impl AppState {
    pub fn new(options: SendOptions) -> Result<Self, AppError> {
        Ok(Self {
            sender: Sender::new(options)?,
            in_flight: InFlight::default(),
        })
    }
}
