//! `whimbrel mcp`: the arguments, and serving over stdio. Only MCP messages go to stdout;
//! a failure to start is written to stderr.

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::ServiceExt;
use wbl_engine::secrets::{SERVICE, Secrets};

use crate::error::McpError;
use crate::server::Server;

pub const USAGE: &str = "Usage: whimbrel mcp --collection <folder> [--collection <folder>]...

Serves the named Whimbrel collections to an AI tool over the Model Context Protocol (stdio).
Secret values stay in the keychain and are never sent to the tool.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Serve(Vec<PathBuf>),
}

/// Reads the arguments after `mcp`.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut folders = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--collection" | "-c" => match args.next() {
                Some(folder) => folders.push(PathBuf::from(folder)),
                None => return Err("--collection needs a folder".into()),
            },
            other => match other.strip_prefix("--collection=") {
                Some(folder) => folders.push(PathBuf::from(folder)),
                None => return Err(format!("Unknown argument {other}")),
            },
        }
    }
    if folders.is_empty() {
        return Err("Name at least one collection with --collection <folder>".into());
    }
    Ok(Command::Serve(folders))
}

/// Serves `folders` over stdin and stdout until the client closes the connection.
pub async fn serve_stdio(folders: &[PathBuf]) -> Result<(), McpError> {
    let keychain = roost_keychain::KeychainSecretStore::open(SERVICE)
        .map_err(|e| McpError::Engine(wbl_engine::EngineError::new("keychain", e.0)))?;
    let server = Server::new(folders, Secrets::new(Arc::new(keychain)))?;
    let running = server.serve(rmcp::transport::stdio()).await.map_err(|e| {
        McpError::InvalidArgument(format!("The client did not start a session: {e}"))
    })?;
    running
        .waiting()
        .await
        .map_err(|e| McpError::Engine(wbl_engine::EngineError::internal(e.to_string())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn collections_are_named_one_flag_each() {
        assert_eq!(
            parse(args(&["--collection", "/a", "-c", "/b", "--collection=/c"])),
            Ok(Command::Serve(vec!["/a".into(), "/b".into(), "/c".into()]))
        );
        assert_eq!(parse(args(&["--help"])), Ok(Command::Help));
    }

    #[test]
    fn bad_arguments_say_what_is_wrong() {
        assert!(parse(args(&[])).unwrap_err().contains("at least one"));
        assert!(
            parse(args(&["--collection"]))
                .unwrap_err()
                .contains("needs a folder")
        );
        assert!(
            parse(args(&["--port", "1"]))
                .unwrap_err()
                .contains("--port")
        );
    }
}
