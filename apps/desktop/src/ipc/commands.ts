import type {
  Api,
  UpdateInfo,
  UpdateProgress,
  HttpExportArgs,
  InteropImportArgs,
  InteropImported,
  InteropSummary,
  ScopeVariable,
  Plan,
  SpecReimportArgs,
  SpecValidateArgs,
  Validation,
  RunStartArgs,
  RunSummary,
  StepResult,
  RuntimeVariable,
  Sent,
  SpecImportArgs,
  SpecImported,
  SpecReadArgs,
  SpecSummary,
  StoredSpec,
  Auth,
  CollectionTls,
  DeviceCode,
  OAuthStatus,
  TokenChange,
  CollectionView,
  CurlImported,
  HistoryEntry,
  HistorySummary,
  ResponseSaveArgs,
  StreamEvent,
  Environment,
  EnvironmentEntry,
  EnvironmentView,
  Example,
  RequestSendArgs,
  ReadRequest,
  Request,
  RequestCreateArgs,
  NodeMoveArgs,
  RequestSaveArgs,
  StoredRequest,
} from "./generated";
import { Channel } from "@tauri-apps/api/core";

import { call, callWith } from "./invoke";

/** Sends the tab's request with its collection's variables, environment and example; rejects
 * with an `AppError` (`cancelled` when `requestCancel` stops it, `unresolved` naming missing
 * variables). */
export function requestSend(
  args: RequestSendArgs,
  onEvent: (event: StreamEvent) => void = () => undefined,
): Promise<Sent> {
  const channel = new Channel<StreamEvent>();
  channel.onmessage = onEvent;
  return callWith("request_send", args, { onEvent: channel });
}

/** Writes a body to `destination`: a spilled one from its file, a shown one from its text or
 * base64. */
export const responseSave = (args: ResponseSaveArgs): Promise<null> => call("response_save", args);

export const historyList = (
  query: string,
  limit: number,
  offset: number,
): Promise<HistorySummary[]> => call("history_list", { query, limit, offset });

export const historyGet = (id: number): Promise<HistoryEntry> => call("history_get", { id });

export const historyClear = (): Promise<null> => call("history_clear");

/** The OAuth 2 token state of the request's effective auth (never the token). */
export const oauthStatus = (args: RequestSendArgs): Promise<OAuthStatus> =>
  call("oauth_status", args);

/** Gets a token by the configured grant and keeps it, as the one in use, under `name` (a
 * default name when `null`); the device code, if any, comes to `onDevice`.
 * `requestCancel("oauth:<requestId>")` stops it. */
export function oauthAuthorize(
  args: RequestSendArgs,
  name: string | null,
  onDevice: (device: DeviceCode) => void,
): Promise<OAuthStatus> {
  const channel = new Channel<DeviceCode>();
  channel.onmessage = onDevice;
  return callWith("oauth_authorize", args, { name, onDevice: channel });
}

/** Renames, uses or removes one of the kept tokens of the request's effective auth. */
export const oauthTokenChange = (
  args: RequestSendArgs,
  change: TokenChange,
): Promise<OAuthStatus> => callWith("oauth_token_change", args, { change });

/** A folder's auth (`path`), or the collection's (`path` empty). */
export const nodeAuthRead = (root: string, path: string): Promise<Auth> =>
  call("node_auth_read", { root, path });

export const nodeAuthSave = (root: string, path: string, auth: Auth): Promise<null> =>
  call("node_auth_save", { root, path, auth });

export const collectionTlsRead = (root: string): Promise<CollectionTls> =>
  call("collection_tls_read", { root });

export const collectionTlsSave = (root: string, tls: CollectionTls): Promise<null> =>
  call("collection_tls_save", { root, tls });

/** The resolved request as a cURL command; secret values only when `reveal`. */
export const curlExport = (send: RequestSendArgs, reveal: boolean): Promise<string> =>
  call("curl_export", { send, reveal });

export const curlImport = (text: string): Promise<CurlImported> => call("curl_import", { text });

/** The values at an RFC 9535 JSONPath in a JSON body, as a pretty JSON array (task 9.4). */
export const responseSelect = (body: string, path: string): Promise<string> =>
  call("response_select", { body, path });

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

/** Moves a request or a folder into `folder` (`""`: the collection), last; returns the new path. */
export const nodeMove = (args: NodeMoveArgs): Promise<string> => call("node_move", args);

export const nodeDelete = (root: string, path: string): Promise<null> =>
  call("node_delete", { root, path });

export type { StoredRequest };

/** Reads an OpenAPI document and says what importing it would make (FR-6). */
export const specRead = (args: SpecReadArgs): Promise<SpecSummary> => call("spec_read", args);

export const specImport = (args: SpecImportArgs): Promise<SpecImported> =>
  call("spec_import", args);

/** The specs imported into an open collection. */
export const specList = (root: string): Promise<StoredSpec[]> => call("spec_list", { root });

/** The model of a spec kept in a collection, for the spec browser. */
export const specOpen = (root: string, dir: string): Promise<Api> =>
  call("spec_open", { root, dir });

/** The runtime variables captures set in a collection (no root: loose requests). */
export const runtimeList = (root?: string): Promise<RuntimeVariable[]> =>
  call("runtime_list", root === undefined ? {} : { root });

export const runtimeClear = (root?: string): Promise<null> =>
  call("runtime_clear", root === undefined ? {} : { root });

/** Runs steps one after the other (FR-6.3); `onStep` gets each result as it is known. */
export function runStart(
  args: RunStartArgs,
  onStep: (result: StepResult) => void,
): Promise<RunSummary> {
  const channel = new Channel<StepResult>();
  channel.onmessage = onStep;
  return callWith("run_start", args, { onStep: channel });
}

export const runCancel = (runId: string): Promise<boolean> => call("run_cancel", { runId });

/** Checks a response against its operation's schema in a kept spec (FR-7.6). */
export const specValidate = (args: SpecValidateArgs): Promise<Validation> =>
  call("spec_validate", args);

/** What re-importing a kept spec from `source` would change (FR-7.5). */
export const specReimportPlan = (args: SpecReimportArgs): Promise<Plan> =>
  call("spec_reimport_plan", args);

export const specReimportApply = (args: SpecReimportArgs): Promise<Plan> =>
  call("spec_reimport_apply", args);

/** The variables a request can use, each once, with where it comes from (FR-2.4). */
export const variablesInScope = (args: RequestSendArgs): Promise<ScopeVariable[]> =>
  call("variables_in_scope", args);

/** Reads a Postman collection or environment, or a `.http` file, and says what it holds. */
export const interopRead = (path: string): Promise<InteropSummary> =>
  call("interop_read", { path });

export const interopImport = (args: InteropImportArgs): Promise<InteropImported> =>
  call("interop_import", args);

/** Writes a folder's requests as a `.http` file; resolves with how many. */
export const httpExport = (args: HttpExportArgs): Promise<number> => call("http_export", args);

/** Sets a request's or folder's place among its siblings. */
export const nodeOrder = (root: string, path: string, order: number): Promise<null> =>
  call("node_order", { root, path, order });

/** Asks the release server for a newer version; `null` when up to date (FR-9.2). */
export const updateCheck = (): Promise<UpdateInfo | null> => call("update_check");

/** Downloads, verifies and installs the update `updateCheck` found, then restarts the app. */
export function updateInstall(onProgress: (progress: UpdateProgress) => void): Promise<void> {
  const channel = new Channel<UpdateProgress>();
  channel.onmessage = onProgress;
  return callWith("update_install", {}, { onProgress: channel });
}
