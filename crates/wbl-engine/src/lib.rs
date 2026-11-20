//! Whimbrel's send pipeline without the app (D-H23): the variables a request sees (with secrets
//! from the keychain), its auth and OAuth 2 tokens, sending, captures and assertions, and the
//! folder runner. The desktop app and the MCP server both send through it.

pub mod args;
pub mod auth;
pub mod checks;
pub mod deliver;
pub mod error;
pub mod interop;
pub mod mask;
pub mod oauth;
pub mod resolve;
pub mod runner;
pub mod runtime;
pub mod secrets;
pub mod send;
pub mod spec;

pub use args::RequestSendArgs;
pub use error::EngineError;

#[cfg(test)]
mod test_support;
