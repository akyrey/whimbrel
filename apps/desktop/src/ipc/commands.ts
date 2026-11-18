import type {
  CollectionView,
  Environment,
  EnvironmentEntry,
  EnvironmentView,
  Example,
  RequestSendArgs,
  ReadRequest,
  Request,
  RequestCreateArgs,
  RequestMoveArgs,
  RequestSaveArgs,
  Response,
  StoredRequest,
} from "./generated";
import { call } from "./invoke";

/** Sends the tab's request with its collection's variables, environment and example; rejects
 * with an `AppError` (`cancelled` when `requestCancel` stops it, `unresolved` naming missing
 * variables). */
export const requestSend = (args: RequestSendArgs): Promise<Response> => call("request_send", args);

/** What would be sent, every secret masked. */
export const requestPreview = (args: RequestSendArgs): Promise<Request> =>
  call("request_preview", args);

export const exampleApply = (request: Request, example: Example): Promise<Request> =>
  call("example_apply", { request, example });

/** Rejects with `invalid_example` when the edits change the method or path. */
export const exampleOverlay = (
  base: Request,
  draft: Request,
  id: string,
  name: string,
): Promise<Example> => call("example_overlay", { base, draft, id, name });

export const environmentList = (root: string): Promise<EnvironmentEntry[]> =>
  call("environment_list", { root });

export const environmentRead = (root: string, id: string): Promise<EnvironmentView> =>
  call("environment_read", { root, id });

export const environmentCreate = (root: string, environment: Environment): Promise<string> =>
  call("environment_create", { root, environment });

export const environmentSave = (
  root: string,
  id: string,
  environment: Environment,
  hash: string,
): Promise<string> => call("environment_save", { root, id, environment, hash });

export const environmentDelete = (root: string, id: string): Promise<null> =>
  call("environment_delete", { root, id });

/** The user's own values for environment `id`, or the collection's variables without one. */
export const variablesSave = (
  root: string,
  id: string | null,
  variables: Record<string, string>,
): Promise<null> =>
  call("variables_save", id === null ? { root, variables } : { root, id, variables });

export const collectionVariablesRead = (root: string): Promise<Record<string, string>> =>
  call("collection_variables_read", { root });

/** The value goes to the keychain; nothing ever reads it back. */
export const secretSet = (
  root: string,
  environment: string,
  name: string,
  value: string,
): Promise<null> => call("secret_set", { root, environment, name, value });

export const secretClear = (root: string, environment: string, name: string): Promise<null> =>
  call("secret_clear", { root, environment, name });

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
