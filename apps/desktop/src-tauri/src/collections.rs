//! Collection commands: plain functions over [`Collections`] (tested without a running app),
//! and the Tauri wrappers that log them and emit watcher events.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;
use wbl_format::collection::{self, Collection, ReadRequest};

use crate::error::AppError;
use crate::ipc::{
    CollectionArgs, CollectionCreateArgs, NodeArgs, NodeNameArgs, NodeOrderArgs, RequestCreateArgs,
    RequestMoveArgs, RequestSaveArgs,
};
use crate::logging::logged;
use crate::watch::{CollectionChanged, Watchers};

/// The open collections. Commands only touch collections the user opened or created.
#[derive(Default)]
pub struct Collections {
    open: Mutex<HashSet<PathBuf>>,
    pub watchers: Watchers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct CollectionView {
    /// The folder, as the commands expect it back.
    pub root: String,
    pub collection: Collection,
}

fn poisoned() -> AppError {
    AppError::internal("The collection list is unusable. Restart Whimbrel and try again.")
}

fn canonical(root: &str) -> Result<PathBuf, AppError> {
    Path::new(root)
        .canonicalize()
        .map_err(|e| AppError::new("not_found", format!("Cannot open the folder {root}: {e}")))
}

impl Collections {
    /// Loads the collection in `root` and remembers it as open.
    pub fn open(&self, root: &str) -> Result<(PathBuf, CollectionView), AppError> {
        let path = canonical(root)?;
        let view = view(&path)?;
        self.open
            .lock()
            .map_err(|_| poisoned())?
            .insert(path.clone());
        Ok((path, view))
    }

    /// Makes `root` a new collection named `name`, and opens it.
    pub fn create(
        &self,
        args: &CollectionCreateArgs,
    ) -> Result<(PathBuf, CollectionView), AppError> {
        collection::create(&canonical(&args.root)?, &args.name)?;
        self.open(&args.root)
    }

    pub fn close(&self, root: &str) -> Result<PathBuf, AppError> {
        let path = self.opened(root)?;
        self.open.lock().map_err(|_| poisoned())?.remove(&path);
        Ok(path)
    }

    /// `root` if it is open; anything else is refused.
    pub fn opened(&self, root: &str) -> Result<PathBuf, AppError> {
        let path = canonical(root)?;
        if self.open.lock().map_err(|_| poisoned())?.contains(&path) {
            Ok(path)
        } else {
            Err(AppError::new("not_open", "Open the collection first"))
        }
    }
}

fn view(root: &Path) -> Result<CollectionView, AppError> {
    Ok(CollectionView {
        root: root.to_string_lossy().into_owned(),
        collection: collection::load(root)?,
    })
}

pub fn tree(state: &Collections, args: &CollectionArgs) -> Result<CollectionView, AppError> {
    view(&state.opened(&args.root)?)
}

pub fn read(state: &Collections, args: &NodeArgs) -> Result<ReadRequest, AppError> {
    Ok(collection::read_request(
        &state.opened(&args.root)?,
        &args.path,
    )?)
}

pub fn save(state: &Collections, args: &RequestSaveArgs) -> Result<String, AppError> {
    let root = state.opened(&args.root)?;
    Ok(collection::save_request(
        &root,
        &args.path,
        &args.stored,
        &args.hash,
    )?)
}

pub fn create_request(state: &Collections, args: &RequestCreateArgs) -> Result<String, AppError> {
    Ok(collection::create_request(
        &state.opened(&args.root)?,
        &args.folder,
        &args.stored,
    )?)
}

pub fn create_folder(state: &Collections, args: &NodeNameArgs) -> Result<String, AppError> {
    Ok(collection::create_folder(
        &state.opened(&args.root)?,
        &args.path,
        &args.name,
    )?)
}

pub fn rename(state: &Collections, args: &NodeNameArgs) -> Result<String, AppError> {
    Ok(collection::rename_request(
        &state.opened(&args.root)?,
        &args.path,
        &args.name,
    )?)
}

pub fn duplicate(state: &Collections, args: &NodeArgs) -> Result<String, AppError> {
    Ok(collection::duplicate_request(
        &state.opened(&args.root)?,
        &args.path,
    )?)
}

pub fn move_to(state: &Collections, args: &RequestMoveArgs) -> Result<String, AppError> {
    Ok(collection::move_request(
        &state.opened(&args.root)?,
        &args.path,
        &args.folder,
    )?)
}

pub fn order(state: &Collections, args: &NodeOrderArgs) -> Result<(), AppError> {
    Ok(collection::set_order(
        &state.opened(&args.root)?,
        &args.path,
        args.order,
    )?)
}

pub fn delete(state: &Collections, args: &NodeArgs) -> Result<(), AppError> {
    Ok(collection::delete(&state.opened(&args.root)?, &args.path)?)
}

/// The event the frontend listens to.
pub const CHANGED_EVENT: &str = "collection_changed";

fn follow(app: &AppHandle, state: &Collections, root: &Path) -> Result<(), AppError> {
    let app = app.clone();
    state
        .watchers
        .watch(root, move |changed: CollectionChanged| {
            // A closed window cannot be told; nothing else to do.
            let _ = app.emit(CHANGED_EVENT, changed);
        })
}

#[tauri::command]
pub async fn collection_open(
    app: AppHandle,
    state: State<'_, Collections>,
    args: CollectionArgs,
) -> Result<CollectionView, AppError> {
    logged("collection_open", async {
        let (root, view) = state.open(&args.root)?;
        follow(&app, &state, &root)?;
        Ok(view)
    })
    .await
}

#[tauri::command]
pub async fn collection_create(
    app: AppHandle,
    state: State<'_, Collections>,
    args: CollectionCreateArgs,
) -> Result<CollectionView, AppError> {
    logged("collection_create", async {
        let (root, view) = state.create(&args)?;
        follow(&app, &state, &root)?;
        Ok(view)
    })
    .await
}

#[tauri::command]
pub async fn collection_close(
    state: State<'_, Collections>,
    args: CollectionArgs,
) -> Result<(), AppError> {
    logged("collection_close", async {
        let root = state.close(&args.root)?;
        state.watchers.unwatch(&root)
    })
    .await
}

/// Declares a command that runs one of the functions above, logged by name.
macro_rules! command {
    ($name:ident, $args:ty, $out:ty, $run:path) => {
        #[tauri::command]
        pub async fn $name(state: State<'_, Collections>, args: $args) -> Result<$out, AppError> {
            logged(stringify!($name), async { $run(&state, &args) }).await
        }
    };
}

command!(collection_tree, CollectionArgs, CollectionView, tree);
command!(request_read, NodeArgs, ReadRequest, read);
command!(request_save, RequestSaveArgs, String, save);
command!(request_create, RequestCreateArgs, String, create_request);
command!(folder_create, NodeNameArgs, String, create_folder);
command!(request_rename, NodeNameArgs, String, rename);
command!(request_duplicate, NodeArgs, String, duplicate);
command!(request_move, RequestMoveArgs, String, move_to);
command!(node_order, NodeOrderArgs, (), order);
command!(node_delete, NodeArgs, (), delete);

#[cfg(test)]
mod tests;
