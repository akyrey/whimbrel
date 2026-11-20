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
    CollectionArgs, CollectionCreateArgs, NodeArgs, NodeMoveArgs, NodeNameArgs, NodeOrderArgs,
    RequestCreateArgs, RequestSaveArgs,
};
use crate::logging::logged;
use crate::resolve::guard;
use crate::secrets::Secrets;
use crate::watch::{CollectionChanged, Watchers};

/// The open collections. Commands only touch collections the user opened or created.
#[derive(Default)]
pub struct Collections {
    open: Mutex<HashSet<PathBuf>>,
    pub watchers: Watchers,
    /// Captured values per collection (keyed by its canonical root).
    pub runtime: crate::runtime::Runtime,
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
        self.runtime.clear(&path.to_string_lossy())?;
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

/// The request's text as it would be written, for the secret guard.
fn text_of(stored: &wbl_format::StoredRequest) -> String {
    wbl_format::RequestFile::new(stored).to_string()
}

pub fn save(
    state: &Collections,
    secrets: &Secrets,
    args: &RequestSaveArgs,
) -> Result<String, AppError> {
    let root = state.opened(&args.root)?;
    guard(&root, &text_of(&args.stored), secrets)?;
    Ok(collection::save_request(
        &root,
        &args.path,
        &args.stored,
        &args.hash,
    )?)
}

pub fn create_request(
    state: &Collections,
    secrets: &Secrets,
    args: &RequestCreateArgs,
) -> Result<String, AppError> {
    let root = state.opened(&args.root)?;
    guard(&root, &text_of(&args.stored), secrets)?;
    Ok(collection::create_request(
        &root,
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

/// Renames a request (and its file; returns the new path) or a folder (its name only; the
/// path stays).
pub fn rename(state: &Collections, args: &NodeNameArgs) -> Result<String, AppError> {
    let root = state.opened(&args.root)?;
    if root.join(&args.path).is_dir() && !args.path.is_empty() {
        collection::rename_folder(&root, &args.path, &args.name)?;
        return Ok(args.path.clone());
    }
    Ok(collection::rename_request(&root, &args.path, &args.name)?)
}

pub fn duplicate(state: &Collections, args: &NodeArgs) -> Result<String, AppError> {
    Ok(collection::duplicate_request(
        &state.opened(&args.root)?,
        &args.path,
    )?)
}

pub fn move_to(state: &Collections, args: &NodeMoveArgs) -> Result<String, AppError> {
    Ok(collection::move_node(
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

pub fn auth_read(state: &Collections, args: &NodeArgs) -> Result<wbl_core::auth::Auth, AppError> {
    Ok(wbl_format::collection::auth::auth_of(
        &state.opened(&args.root)?,
        &args.path,
    )?)
}

/// Saves a folder's or the collection's auth, refusing literal secret values like a request.
pub fn auth_save(
    state: &Collections,
    secrets: &Secrets,
    args: &crate::ipc::AuthSaveArgs,
) -> Result<(), AppError> {
    let root = state.opened(&args.root)?;
    let text = serde_json::to_string(&args.auth).map_err(|e| AppError::internal(e.to_string()))?;
    guard(&root, &text, secrets)?;
    Ok(wbl_format::collection::auth::save_auth(
        &root, &args.path, &args.auth,
    )?)
}

pub fn headers_read(
    state: &Collections,
    args: &NodeArgs,
) -> Result<Vec<wbl_core::Header>, AppError> {
    Ok(wbl_format::collection::headers::headers_of(
        &state.opened(&args.root)?,
        &args.path,
    )?)
}

/// Saves the headers a folder or the collection adds, refusing literal secret values.
pub fn headers_save(
    state: &Collections,
    secrets: &Secrets,
    args: &crate::ipc::HeadersSaveArgs,
) -> Result<(), AppError> {
    let root = state.opened(&args.root)?;
    let text =
        serde_json::to_string(&args.headers).map_err(|e| AppError::internal(e.to_string()))?;
    guard(&root, &text, secrets)?;
    Ok(wbl_format::collection::headers::save_headers(
        &root,
        &args.path,
        &args.headers,
    )?)
}

pub fn tls_read(
    state: &Collections,
    args: &CollectionArgs,
) -> Result<wbl_format::CollectionTls, AppError> {
    Ok(wbl_format::collection::auth::tls(
        &state.opened(&args.root)?,
    )?)
}

pub fn tls_save(state: &Collections, args: &crate::ipc::TlsSaveArgs) -> Result<(), AppError> {
    Ok(wbl_format::collection::auth::save_tls(
        &state.opened(&args.root)?,
        &args.tls,
    )?)
}

#[tauri::command]
pub async fn node_auth_save(
    state: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: crate::ipc::AuthSaveArgs,
) -> Result<(), AppError> {
    logged("node_auth_save", async {
        auth_save(&state, &secrets, &args)
    })
    .await
}

#[tauri::command]
pub async fn node_headers_save(
    state: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: crate::ipc::HeadersSaveArgs,
) -> Result<(), AppError> {
    logged("node_headers_save", async {
        headers_save(&state, &secrets, &args)
    })
    .await
}

command!(node_auth_read, NodeArgs, wbl_core::auth::Auth, auth_read);
command!(
    node_headers_read,
    NodeArgs,
    Vec<wbl_core::Header>,
    headers_read
);
command!(
    collection_tls_read,
    CollectionArgs,
    wbl_format::CollectionTls,
    tls_read
);
command!(collection_tls_save, crate::ipc::TlsSaveArgs, (), tls_save);
command!(collection_tree, CollectionArgs, CollectionView, tree);
command!(request_read, NodeArgs, ReadRequest, read);
#[tauri::command]
pub async fn request_save(
    state: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestSaveArgs,
) -> Result<String, AppError> {
    logged("request_save", async { save(&state, &secrets, &args) }).await
}

#[tauri::command]
pub async fn request_create(
    state: State<'_, Collections>,
    secrets: State<'_, Secrets>,
    args: RequestCreateArgs,
) -> Result<String, AppError> {
    logged("request_create", async {
        create_request(&state, &secrets, &args)
    })
    .await
}
command!(folder_create, NodeNameArgs, String, create_folder);
command!(request_rename, NodeNameArgs, String, rename);
command!(request_duplicate, NodeArgs, String, duplicate);
command!(node_move, NodeMoveArgs, String, move_to);
command!(node_order, NodeOrderArgs, (), order);
command!(node_delete, NodeArgs, (), delete);

#[cfg(test)]
mod tests;
