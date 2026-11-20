//! `whimbrel mcp …`: the MCP server for AI tools (D-H23), run instead of the window. It runs
//! inside the app binary so it shares the app's keychain access and updates with it.

use std::process::ExitCode;

use wbl_mcp::cli::{Command, USAGE, parse, serve_stdio};

/// Whether the command line asks for the MCP server (`whimbrel mcp …`).
pub fn requested(args: &[String]) -> bool {
    args.get(1).is_some_and(|a| a == "mcp")
}

/// Runs the server with the arguments after `mcp`. Messages for people go to stderr: stdout
/// carries the protocol.
pub fn run(args: Vec<String>) -> ExitCode {
    let folders = match parse(args) {
        Ok(Command::Serve(folders)) => folders,
        Ok(Command::Help) => {
            eprintln!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Whimbrel could not start the MCP server: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(serve_stdio(&folders)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Whimbrel MCP server: {error} ({})", error.code());
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_mcp_subcommand_starts_the_server() {
        let line = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert!(requested(&line(&["whimbrel", "mcp", "--collection", "/a"])));
        assert!(!requested(&line(&["whimbrel"])));
        assert!(!requested(&line(&["whimbrel", "--mcp"])));
    }
}
