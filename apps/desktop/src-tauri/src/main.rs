// Prevents an additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if whimbrel_lib::mcp::requested(&args) {
        return whimbrel_lib::mcp::run(args.into_iter().skip(2).collect());
    }
    match whimbrel_lib::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Whimbrel failed to start: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
