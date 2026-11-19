//! Whimbrel desktop backend: Tauri commands over the HTTP engine.

pub mod auth_apply;
pub mod checks;
pub mod collections;
pub mod commands;
pub mod environments;
pub mod error;
pub mod history;
pub mod interop;
pub mod ipc;
pub mod logging;
pub mod oauth;
pub mod registry;
pub mod resolve;
pub mod runner;
pub mod runtime;
pub mod secrets;
pub mod spec;
pub mod state;
pub mod watch;

#[cfg(test)]
mod bindings;
#[cfg(test)]
mod test_support;

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
        .plugin(tauri_plugin_opener::init())
        .manage(collections::Collections::default())
        .manage(runner::Runs::default())
        .setup(|app| {
            let spill_dir = app.path().app_cache_dir()?.join("bodies");
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let history = wbl_store::History::open(&data.join("whimbrel.db"))
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let state = AppState::new(SendOptions {
                spill_dir: Some(spill_dir),
                ..SendOptions::default()
            })
            .map(|s| s.with_history(history))
            .map_err(|e| std::io::Error::other(e.message))?;
            app.manage(state);
            let keychain = roost_keychain::KeychainSecretStore::open(secrets::SERVICE)
                .map_err(|e| std::io::Error::other(e.0))?;
            app.manage(secrets::Secrets::new(std::sync::Arc::new(keychain)));
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
            commands::request_preview,
            commands::variables_in_scope,
            commands::response_save,
            commands::curl_export,
            oauth::oauth_status,
            oauth::oauth_authorize,
            oauth::oauth_forget,
            commands::curl_import,
            history::history_list,
            history::history_get,
            history::history_clear,
            commands::example_apply,
            commands::example_overlay,
            environments::environment_list,
            environments::environment_read,
            environments::environment_create,
            environments::environment_save,
            environments::environment_delete,
            environments::variables_save,
            environments::collection_variables_read,
            environments::secret_set,
            environments::secret_clear,
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
            collections::node_auth_read,
            collections::node_auth_save,
            collections::collection_tls_read,
            collections::collection_tls_save,
            spec::spec_read,
            spec::spec_import,
            spec::spec_list,
            spec::spec_open,
            spec::spec_validate,
            spec::spec_reimport_plan,
            spec::spec_reimport_apply,
            interop::interop_read,
            interop::interop_import,
            interop::http_export,
            runtime::runtime_list,
            runtime::runtime_clear,
            runner::run_start,
            runner::run_cancel,
        ])
        .run(tauri::generate_context!())
}
