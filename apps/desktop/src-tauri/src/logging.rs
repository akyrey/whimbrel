//! Log files: JSON lines in the app log folder, one file a day, the last week kept, through
//! `roost-shell`.
//!
//! Only command names, durations and error codes are logged: never URLs, headers, bodies or
//! secrets. `logged` records just the command name, so a
//! command's arguments cannot reach the log by accident.

use std::future::Future;
use std::path::Path;

use roost_shell::logging::{self, ErrorCode, LogConfig};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{InitError, RollingFileAppender};

pub use roost_shell::logging::{mark_start, ready};

use crate::error::AppError;

const CONFIG: LogConfig = LogConfig {
    file_prefix: "whimbrel",
    app_target: "whimbrel_lib",
};

impl ErrorCode for AppError {
    fn code(&self) -> &str {
        &self.code
    }
}

/// The appender for `dir`: daily files, the oldest removed after a week.
pub fn appender(dir: &Path) -> Result<RollingFileAppender, InitError> {
    logging::appender(dir, CONFIG.file_prefix)
}

/// Starts logging to `dir`. Keep the guard for the life of the app: dropping it flushes and
/// stops the writer thread.
pub fn init(dir: &Path) -> Result<WorkerGuard, InitError> {
    logging::init(dir, CONFIG)
}

/// Runs a command and logs its name, how long it took and, on failure, the error code.
pub async fn logged<T>(
    command: &'static str,
    call: impl Future<Output = Result<T, AppError>>,
) -> Result<T, AppError> {
    logging::logged(command, call).await
}

#[cfg(test)]
mod tests;
