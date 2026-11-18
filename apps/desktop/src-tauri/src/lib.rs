//! Whimbrel desktop backend: Tauri commands over the HTTP engine.

pub mod collections;
pub mod commands;
pub mod error;
pub mod ipc;
pub mod logging;
pub mod registry;
pub mod state;
pub mod watch;

#[cfg(test)]
mod bindings;

use tauri::Manager;
use wbl_http::SendOptions;

use crate::state::AppState;

struct LogGuard {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

/// Starts the app. Returns an error instead of panicking if setup fails.
pub fn run() -> Result<(), tauri::Error> {
    logging::mark_start();
    tauri::Builder::default()
        // Size, position and maximised or full screen state.
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED
                        | tauri_plugin_window_state::StateFlags::FULLSCREEN,
                )
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .manage(collections::Collections::default())
        .setup(|app| {
            let state = AppState::new(SendOptions::default())
                .map_err(|e| std::io::Error::other(e.message))?;
            app.manage(state);
            let logs = app.path().app_log_dir()?;
            std::fs::create_dir_all(&logs)?;
            let guard = logging::init(&logs).map_err(|e| std::io::Error::other(e.to_string()))?;
            // Kept for the life of the app so buffered lines are written.
            app.manage(LogGuard { _guard: guard });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::request_send,
            commands::request_cancel,
            collections::collection_open,
            collections::collection_create,
            collections::collection_close,
            collections::collection_tree,
            collections::request_read,
            collections::request_save,
            collections::request_create,
            collections::folder_create,
            collections::request_rename,
            collections::request_duplicate,
            collections::request_move,
            collections::node_order,
            collections::node_delete,
        ])
        .run(tauri::generate_context!())
}
