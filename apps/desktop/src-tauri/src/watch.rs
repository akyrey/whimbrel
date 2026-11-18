//! Follows changes to open collections made outside the app (git checkout, pull, an editor).
//! Events are debounced and reported as paths relative to the collection. The app's own saves
//! are reported too; the frontend recognises them by the hash its tab holds.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppError;

/// How long a burst of changes (a checkout) is gathered into one event.
const DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CollectionChanged {
    pub root: String,
    /// Changed files and folders, relative, with `/`; sorted, without duplicates.
    pub paths: Vec<String>,
}

/// The paths worth reporting: inside `root`, not hidden (`.git`, the writer's `.x.tmp` files).
pub fn reportable(root: &Path, changed: &[PathBuf]) -> Vec<String> {
    let mut paths: Vec<String> = changed
        .iter()
        .filter_map(|p| p.strip_prefix(root).ok())
        .filter(|p| !p.as_os_str().is_empty())
        .filter(|p| {
            !p.components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        })
        .map(|p| {
            p.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

type Watcher = Debouncer<RecommendedWatcher, RecommendedCache>;

/// One watcher per open collection.
#[derive(Default)]
pub struct Watchers(Mutex<HashMap<PathBuf, Watcher>>);

fn poisoned() -> AppError {
    AppError::internal("The file watcher is unusable. Restart Whimbrel and try again.")
}

impl Watchers {
    /// Watches `root` (again: a second call replaces the first) and calls `sink` with each
    /// batch of changes.
    pub fn watch(
        &self,
        root: &Path,
        sink: impl Fn(CollectionChanged) + Send + 'static,
    ) -> Result<(), AppError> {
        let base = root.to_path_buf();
        let name = root.to_string_lossy().into_owned();
        let handler = move |result: DebounceEventResult| {
            // Watcher errors (a removed folder) only mean fewer events; the tree is reloaded
            // from disk on the next change or when reopened.
            let Ok(events) = result else { return };
            let changed: Vec<PathBuf> = events.iter().flat_map(|e| e.event.paths.clone()).collect();
            let paths = reportable(&base, &changed);
            if !paths.is_empty() {
                sink(CollectionChanged {
                    root: name.clone(),
                    paths,
                });
            }
        };
        let mut watcher = new_debouncer(DEBOUNCE, None, handler).map_err(watch_failed)?;
        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(watch_failed)?;
        self.0
            .lock()
            .map_err(|_| poisoned())?
            .insert(root.to_path_buf(), watcher);
        Ok(())
    }

    pub fn unwatch(&self, root: &Path) -> Result<(), AppError> {
        self.0.lock().map_err(|_| poisoned())?.remove(root);
        Ok(())
    }
}

fn watch_failed(error: notify::Error) -> AppError {
    AppError::new(
        "watch_failed",
        format!("Cannot follow changes to the collection: {error}"),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::mpsc;

    use super::*;

    #[test]
    fn reports_relative_visible_paths_once() {
        let root = Path::new("/c");
        let changed = [
            PathBuf::from("/c/users/list.req.toml"),
            PathBuf::from("/c/users/list.req.toml"),
            PathBuf::from("/c/.git/index"),
            PathBuf::from("/c/users/.list.req.toml.tmp"),
            PathBuf::from("/elsewhere/x"),
            PathBuf::from("/c"),
            PathBuf::from("/c/a.req.toml"),
        ];

        assert_eq!(
            reportable(root, &changed),
            vec!["a.req.toml", "users/list.req.toml"]
        );
    }

    #[test]
    fn reports_a_file_written_in_the_collection() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let (tx, rx) = mpsc::channel();
        let watchers = Watchers::default();
        watchers
            .watch(&root, move |changed| {
                let _ = tx.send(changed);
            })
            .unwrap();

        std::fs::write(root.join("ping.req.toml"), "name = \"x\"").unwrap();

        let changed = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            changed.paths.contains(&"ping.req.toml".to_owned()),
            "{changed:?}"
        );
        watchers.unwatch(&root).unwrap();
    }
}
