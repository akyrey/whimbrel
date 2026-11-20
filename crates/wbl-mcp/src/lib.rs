//! Whimbrel's MCP server (FR-10, D-H23): AI tools read and change collections and
//! environments, and send requests, the way the app does, without ever seeing a secret value.
//! The app binary runs it as `whimbrel mcp --collection <folder>…` over stdio.

pub mod error;
pub mod reachable;
pub mod server;
mod tools;

pub use error::McpError;
pub use server::Server;

#[cfg(test)]
mod test_support;
