//! Whimbrel's model: what a request is, independent of how it is stored or sent. No I/O.

pub mod example;
pub mod interpolate;
pub mod request;
pub mod settings;

pub use request::{Body, BodyKind, Header, Request, RequestError};
pub use settings::RequestSettings;
