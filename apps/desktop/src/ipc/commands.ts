import type {
  CollectionView,
  ReadRequest,
  Request,
  RequestCreateArgs,
  RequestMoveArgs,
  RequestSaveArgs,
  Response,
  StoredRequest,
} from "./generated";
import { call } from "./invoke";

/** Sends `request` for the tab `requestId`; rejects with an `AppError` (code `cancelled` when
 * `requestCancel` stops it). */
export const requestSend = (requestId: string, request: Request): Promise<Response> =>
  call("request_send", { requestId, request });

/** Stops the tab's request; `false` when it had already finished. */
export const requestCancel = (requestId: string): Promise<boolean> =>
  call("request_cancel", { requestId });

export const collectionOpen = (root: string): Promise<CollectionView> =>
  call("collection_open", { root });

export const collectionCreate = (root: string, name: string): Promise<CollectionView> =>
  call("collection_create", { root, name });

export const collectionClose = (root: string): Promise<null> => call("collection_close", { root });

export const collectionTree = (root: string): Promise<CollectionView> =>
  call("collection_tree", { root });

export const requestRead = (root: string, path: string): Promise<ReadRequest> =>
  call("request_read", { root, path });

/** Rejects with `conflict` when the file changed since `hash`; resolves to the new hash. */
export const requestSave = (args: RequestSaveArgs): Promise<string> => call("request_save", args);

export const requestCreate = (args: RequestCreateArgs): Promise<string> =>
  call("request_create", args);

export const folderCreate = (root: string, parent: string, name: string): Promise<string> =>
  call("folder_create", { root, path: parent, name });

export const requestRename = (root: string, path: string, name: string): Promise<string> =>
  call("request_rename", { root, path, name });

export const requestDuplicate = (root: string, path: string): Promise<string> =>
  call("request_duplicate", { root, path });

export const requestMove = (args: RequestMoveArgs): Promise<string> => call("request_move", args);

export const nodeDelete = (root: string, path: string): Promise<null> =>
  call("node_delete", { root, path });

export type { StoredRequest };
