//! Checking for, downloading and installing updates from the GitHub release (FR-9.2), through
//! `roost-shell`. `check` remembers the update it found so `install` installs exactly what the
//! user was shown; the updater verifies every download against the public key in
//! `tauri.conf.json` before installing it.

use roost_shell::updates;
use tauri::State;

pub use roost_shell::updates::{PendingUpdate, UpdateInfo, UpdateProgress};

use crate::error::AppError;
use crate::logging::logged;

/// The update state, named for the one message that asks for a restart.
pub fn pending() -> PendingUpdate {
    PendingUpdate::new("Whimbrel")
}

/// Asks the release server for a newer version and remembers it. `None` means up to date.
#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfo>, AppError> {
    use tauri_plugin_updater::UpdaterExt;
    logged("update_check", async {
        Ok(updates::check(app.updater(), &pending).await?)
    })
    .await
}

/// Downloads, verifies and installs the update `update_check` found, then restarts.
#[tauri::command]
pub async fn update_install(
    app: tauri::AppHandle,
    pending: State<'_, PendingUpdate>,
    on_progress: tauri::ipc::Channel<UpdateProgress>,
) -> Result<(), AppError> {
    let progress = move |step: UpdateProgress| {
        // A closed channel means the window went away; the install itself carries on.
        let _ = on_progress.send(step);
    };
    logged("update_install", async {
        Ok(updates::install(&pending, progress).await?)
    })
    .await?;
    app.restart()
}
