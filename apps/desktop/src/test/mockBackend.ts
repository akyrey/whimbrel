/**
 * An in-memory stand-in for the Rust backend, for Playwright and App tests. It follows the same
 * contract as the real commands (src-tauri/src/commands.rs and collections.rs).
 *
 * Requests: `/echo` answers 200 with JSON describing what was sent; `/slow` waits until
 * cancelled; host `unreachable.test` fails with `connect`; a URL without http(s):// fails with
 * `invalid_request`; anything else answers 404.
 *
 * Collections: an in-memory file system with one collection at `/demo`; the folder dialog
 * answers `/demo`. `externalEdit` changes a file as another program would and emits
 * `collection_changed`, as the watcher does.
 */
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";

import { applyAuth, authCommand, resetAuth } from "./mockAuth";
import { checksFor, resetRuntime, runtimeCommand } from "./mockChecks";
import { resetSpecs, specCommand } from "./mockSpecs";
import { phase4Command, phase4Route, recordHistory, resetHistory } from "./mockExtras";
import {
  apply,
  environmentCommand,
  guard,
  overlay,
  resolveFor,
  variablesFor,
  seedEnvironments,
} from "./mockEnvironments";

import type {
  AppError,
  Example,
  CollectionView,
  Node,
  Request,
  RequestCreateArgs,
  RequestMoveArgs,
  RequestSaveArgs,
  RequestSendArgs,
  Response,
  RunStartArgs,
  RunSummary,
  Sent,
  StepResult,
  StoredRequest,
} from "../ipc/generated";

type File = { stored: StoredRequest; hash: string };
type Fs = { id: string; name: string; files: Map<string, File>; folders: Map<string, string> };

/** Folder orders, by root and path (requests keep theirs in the file). */
const folderOrders = new Map<string, number>();

export const DEMO_ROOT = "/demo";

let version = 0;
const nextHash = () => `h${String((version += 1))}`;
const collections = new Map<string, Fs>();

const get = (stored: Partial<StoredRequest> & { name: string; url?: string }): StoredRequest => ({
  name: stored.name,
  order: stored.order ?? 0,
  request: {
    method: "GET",
    url: stored.url ?? "http://localhost/echo",
    headers: [],
    body: { type: "none" },
  },
  examples: [],
  settings: { timeoutMs: null, followRedirects: true, verifyTls: true },
  auth: { type: "inherit" },
  variables: {},
  captures: [],
  assertions: [],
});

/** Session: a login that captures a token, and a call that needs it (Phase 7). */
const LOGIN: StoredRequest = {
  ...get({ name: "Get a token", order: 10, url: "http://localhost/login" }),
  request: {
    ...get({ name: "Get a token" }).request,
    method: "POST",
    url: "http://localhost/login",
  },
  captures: [{ var: "token", from: "body", path: "$.token", regex: null, secret: true }],
};
const ME: StoredRequest = {
  ...get({ name: "Me", order: 20, url: "http://localhost/me" }),
  request: {
    ...get({ name: "Me" }).request,
    url: "http://localhost/me",
    headers: [{ name: "Authorization", value: "Bearer {{token}}", enabled: true }],
  },
  assertions: [{ check: "status", equals: 200 }],
  examples: [
    {
      id: "forged",
      name: "Forged",
      order: 10,
      variables: { token: "forged" },
      query: [],
      headers: [],
      body: null,
    },
  ],
};

function seed(): void {
  collections.clear();
  folderOrders.clear();
  collections.set(DEMO_ROOT, {
    id: "demo-id",
    name: "Demo API",
    folders: new Map([
      ["users", "Users"],
      ["session", "Session"],
    ]),
    files: new Map([
      [
        "ping.req.toml",
        {
          stored: get({ name: "Ping", order: 10, url: "http://localhost/echo" }),
          hash: nextHash(),
        },
      ],
      [
        "users/list-users.req.toml",
        { stored: get({ name: "List users", order: 10 }), hash: nextHash() },
      ],
      ["session/get-a-token.req.toml", { stored: LOGIN, hash: nextHash() }],
      ["session/me.req.toml", { stored: ME, hash: nextHash() }],
    ]),
  });
}

const fail = (code: string, message: string): Promise<never> =>
  Promise.reject({ code, message } satisfies AppError);

const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "request";

const parentOf = (path: string) => path.split("/").slice(0, -1).join("/");
const join = (folder: string, name: string) => (folder ? `${folder}/${name}` : name);

function unique(fs: Fs, folder: string, name: string, suffix: string): string {
  let candidate = join(folder, `${slug(name)}${suffix}`);
  for (let n = 2; fs.files.has(candidate) || fs.folders.has(candidate); n += 1) {
    candidate = join(folder, `${slug(name)}-${String(n)}${suffix}`);
  }
  return candidate;
}

function nodes(fs: Fs, folder: string): Node[] {
  const folders: Node[] = [...fs.folders]
    .filter(([path]) => parentOf(path) === folder)
    .map(([path, name]) => ({
      kind: "folder",
      path,
      name,
      order: folderOrders.get(`${fs.id}:${path}`) ?? 0,
      children: nodes(fs, path),
    }));
  const files: Node[] = [...fs.files]
    .filter(([path]) => parentOf(path) === folder)
    .map(([path, f]) => ({
      kind: "request",
      path,
      name: f.stored.name,
      order: f.stored.order,
      method: f.stored.request.method,
      examples: f.stored.examples.map((e) => ({ id: e.id, name: e.name })),
    }));
  const key = (n: Node) => (n.kind === "broken" ? 0 : n.order);
  return [...files, ...folders].sort((x, y) => key(x) - key(y));
}

const view = (root: string, fs: Fs): CollectionView => ({
  root,
  collection: { id: fs.id, name: fs.name, children: nodes(fs, "") },
});

function collection(root: string): Fs | null {
  return collections.get(root) ?? null;
}

type Args = Record<string, unknown> & { root: string; path: string; name: string };

/** Writes generated requests into a new folder of an open collection (the spec import). */
export function mockInstall(
  root: string,
  folderName: string,
  requests: { id: string; folder: string; stored: StoredRequest }[],
): { folder: string; files: Record<string, string> } | null {
  const fs = collection(root);
  if (!fs) return null;
  const base = unique(fs, "", folderName, "");
  fs.folders.set(base, folderName);
  const files: Record<string, string> = {};
  for (const r of requests) {
    const dir = join(base, slug(r.folder));
    if (!fs.folders.has(dir)) fs.folders.set(dir, r.folder);
    const path = unique(fs, dir, r.stored.name, ".req.toml");
    fs.files.set(path, { stored: r.stored, hash: nextHash() });
    files[r.id] = path;
  }
  return { folder: base, files };
}

export const mockStored = (name: string, url: string, method = "GET"): StoredRequest => ({
  ...get({ name, url }),
  request: { ...get({ name, url }).request, method },
});

function collectionCommand(command: string, a: Args): Promise<unknown> | null {
  const fs = collection(a.root);
  if (command === "collection_open" || command === "collection_tree") {
    return fs
      ? Promise.resolve(view(a.root, fs))
      : fail("invalid_file", "`collection.toml` is missing");
  }
  if (command === "collection_create") {
    const created: Fs = { id: `id-${a.root}`, name: a.name, files: new Map(), folders: new Map() };
    collections.set(a.root, created);
    return Promise.resolve(view(a.root, created));
  }
  if (command === "collection_close") return Promise.resolve(null);
  if (
    !command.startsWith("request_") &&
    !command.startsWith("folder_") &&
    !command.startsWith("node_")
  ) {
    return null;
  }
  if (!fs) return fail("not_open", "Open the collection first");
  return fileCommand(fs, command, a);
}

function fileCommand(fs: Fs, command: string, a: Args): Promise<unknown> {
  const file = fs.files.get(a.path);
  switch (command) {
    case "request_read":
      return file ? Promise.resolve(file) : fail("io", "No such file");
    case "request_save": {
      const { stored, hash } = a as unknown as RequestSaveArgs;
      if (!file) return fail("io", "No such file");
      if (file.hash !== hash)
        return fail("conflict", "The file changed on disk since it was opened");
      const leak = guard(DEMO_ROOT, JSON.stringify(stored));
      if (leak) return Promise.reject(leak);
      const saved = { stored, hash: nextHash() };
      fs.files.set(a.path, saved);
      return Promise.resolve(saved.hash);
    }
    case "request_create": {
      const { folder, stored } = a as unknown as RequestCreateArgs;
      const leak = guard(DEMO_ROOT, JSON.stringify(stored));
      if (leak) return Promise.reject(leak);
      const path = unique(fs, folder, stored.name, ".req.toml");
      fs.files.set(path, { stored: { ...stored, order: 100 }, hash: nextHash() });
      return Promise.resolve(path);
    }
    case "folder_create": {
      const path = unique(fs, a.path, a.name, "");
      fs.folders.set(path, a.name);
      return Promise.resolve(path);
    }
    case "request_rename": {
      if (fs.folders.has(a.path)) {
        fs.folders.set(a.path, a.name);
        return Promise.resolve(a.path);
      }
      if (!file) return fail("io", "No such file");
      fs.files.delete(a.path);
      const path = unique(fs, parentOf(a.path), a.name, ".req.toml");
      fs.files.set(path, { stored: { ...file.stored, name: a.name }, hash: nextHash() });
      return Promise.resolve(path);
    }
    case "node_order": {
      const order = (a as unknown as { order: number }).order;
      if (fs.folders.has(a.path)) {
        folderOrders.set(`${fs.id}:${a.path}`, order);
      } else if (file) {
        fs.files.set(a.path, { stored: { ...file.stored, order }, hash: nextHash() });
      }
      return Promise.resolve(null);
    }
    case "request_duplicate": {
      if (!file) return fail("io", "No such file");
      const name = `${file.stored.name} copy`;
      const path = unique(fs, parentOf(a.path), name, ".req.toml");
      fs.files.set(path, { stored: { ...file.stored, name }, hash: nextHash() });
      return Promise.resolve(path);
    }
    case "request_move": {
      const { folder } = a as unknown as RequestMoveArgs;
      if (!file) return fail("io", "No such file");
      fs.files.delete(a.path);
      const path = unique(
        fs,
        folder,
        a.path.split("/").pop()?.replace(".req.toml", "") ?? "x",
        ".req.toml",
      );
      fs.files.set(path, file);
      return Promise.resolve(path);
    }
    case "node_delete":
      for (const key of [...fs.files.keys()])
        if (key === a.path || key.startsWith(`${a.path}/`)) fs.files.delete(key);
      for (const key of [...fs.folders.keys()])
        if (key === a.path || key.startsWith(`${a.path}/`)) fs.folders.delete(key);
      return Promise.resolve(null);
    default:
      return Promise.resolve(null);
  }
}

/** Another program changed (or, with `null`, deleted) a file of the demo collection. */
export async function externalEdit(
  path: string,
  change: ((stored: StoredRequest) => StoredRequest) | null,
): Promise<void> {
  const fs = collection(DEMO_ROOT);
  const file = fs?.files.get(path);
  if (!fs || !file) return;
  if (change) fs.files.set(path, { stored: change(file.stored), hash: nextHash() });
  else fs.files.delete(path);
  await emit("collection_changed", { root: DEMO_ROOT, paths: [path] });
}

function response(status: number, reason: string, contentType: string, text: string): Response {
  return {
    status,
    reason,
    httpVersion: "HTTP/1.1",
    url: "",
    headers: [
      { name: "content-type", value: contentType },
      { name: "x-mock", value: "1" },
    ],
    body: { text, base64: null, size: text.length, truncated: false, file: null, contentType },
    timing: { headersMs: 12, totalMs: 15 },
    streamEvents: 0,
  };
}

/** What the echo route reports as the body it received. */
function bodyText(body: Request["body"]): string {
  switch (body.type) {
    case "none":
      return "";
    case "text":
      return body.text;
    case "file":
      return `file ${body.path}`;
    case "form":
    case "multipart":
      return body.fields
        .filter((f) => f.enabled)
        .map((f) => `${f.name}=${f.file ? `@${f.value}` : f.value}`)
        .join("&");
  }
}

function answer(request: Request): Response | AppError {
  if (!/^https?:\/\//i.test(request.url)) {
    return {
      code: "invalid_request",
      message: "The URL needs http:// or https://",
      detail: { field: "url" },
    };
  }
  if (request.url.includes("unreachable.test")) {
    return { code: "connect", message: "Could not connect (connection failed)" };
  }
  const special = phase4Route(request);
  if (special) return special;
  if (request.url.endsWith("/login")) {
    return { ...response(200, "OK", "application/json", '{"token":"tok-123"}'), url: request.url };
  }
  if (request.url.endsWith("/me")) {
    const authorised = request.headers.some(
      (h) => h.enabled && h.name.toLowerCase() === "authorization" && h.value === "Bearer tok-123",
    );
    return authorised
      ? { ...response(200, "OK", "application/json", '{"name":"ada"}'), url: request.url }
      : { ...response(401, "Unauthorized", "application/json", "{}"), url: request.url };
  }
  if (request.url.includes("/echo")) {
    const sent = {
      method: request.method,
      headers: request.headers.filter((h) => h.enabled).map((h) => [h.name, h.value]),
      body: bodyText(request.body),
    };
    return { ...response(200, "OK", "application/json", JSON.stringify(sent)), url: request.url };
  }
  return { ...response(404, "Not Found", "text/plain", "no such route"), url: request.url };
}

const isError = (value: Response | AppError): value is AppError => "code" in value;

const pending = new Map<string, (error: AppError) => void>();

/** request_send and request_preview, as src-tauri/src/commands.rs. */
function mockSend(sendArgs: RequestSendArgs, preview: boolean): Promise<unknown> {
  const variables = resolveFor(sendArgs, preview);
  if ("code" in variables) return Promise.reject(variables);
  const resolved = applyAuth(sendArgs, variables, preview);
  if ("code" in resolved) return Promise.reject(resolved);
  if (preview) return Promise.resolve(resolved);
  if (resolved.url.includes("/slow")) {
    return new Promise((_, reject: (error: AppError) => void) => {
      pending.set(sendArgs.requestId, reject);
    });
  }
  const result = answer(resolved);
  recordHistory(resolveFor(sendArgs, true) as Request, result, sendArgs.name);
  return isError(result)
    ? Promise.reject(result)
    : Promise.resolve({ response: result, checks: checksFor(sendArgs, result) } satisfies Sent);
}

/** run_start, as src-tauri/src/runner.rs: the steps in order, each result streamed. */
async function mockRun(args: RunStartArgs, onStep: (r: StepResult) => void): Promise<RunSummary> {
  const fs = collection(args.root);
  if (!fs) return fail("not_open", "Open the collection first");
  const planned = args.steps.flatMap((step) => {
    const stored = fs.files.get(step.path)?.stored;
    if (!stored) return [];
    const examples =
      step.mode.kind === "base" || stored.examples.length === 0
        ? [null]
        : step.mode.kind === "each_example"
          ? stored.examples
          : stored.examples.filter((e) => step.mode.kind === "example" && e.id === step.mode.id);
    return examples.map((example) => ({ path: step.path, stored, example }));
  });
  const summary: RunSummary = { passed: 0, failed: 0, skipped: 0, cancelled: false };
  for (const [index, p] of planned.entries()) {
    const sendArgs: RequestSendArgs = {
      requestId: `run:${args.runId}`,
      request: p.stored.request,
      root: args.root,
      ...(args.environment === undefined ? {} : { environment: args.environment }),
      ...(p.example ? { example: p.example } : {}),
      variables: p.stored.variables,
      captures: p.stored.captures,
      assertions: p.stored.assertions,
      auth: p.stored.auth,
      name: p.stored.name,
      path: p.path,
    };
    const base = {
      index,
      path: p.path,
      name: p.stored.name,
      example: p.example?.name ?? null,
      method: p.stored.request.method,
    };
    let result: StepResult;
    try {
      const sent = (await mockSend(sendArgs, false)) as Sent;
      const passed =
        sent.checks.assertions.every((a) => a.passed) &&
        sent.checks.captures.every((c) => c.error === null);
      result = {
        ...base,
        status: sent.response.status,
        totalMs: sent.response.timing.totalMs,
        passed,
        error: null,
        checks: sent.checks,
      };
    } catch (caught) {
      const error = (caught as AppError).message;
      result = { ...base, status: null, totalMs: null, passed: false, error, checks: null };
      if ((caught as AppError).code === "cancelled") summary.cancelled = true;
    }
    onStep(result);
    if (result.passed) {
      summary.passed += 1;
      continue;
    }
    summary.failed += 1;
    if (args.stopOnFailure || summary.cancelled) {
      summary.skipped = planned.length - index - 1;
      break;
    }
  }
  return summary;
}

export function installMockBackend(): void {
  seed();
  seedEnvironments(DEMO_ROOT);
  resetHistory();
  resetAuth();
  resetSpecs();
  resetRuntime();
  mockIPC(
    (command, payload) => {
      const args = (payload as { args?: Args } | undefined)?.args ?? ({} as Args);
      if (command === "plugin:dialog|open") return Promise.resolve(DEMO_ROOT);
      if (command === "plugin:dialog|save") return Promise.resolve("/tmp/saved-body");
      if (command === "request_preview" || command === "request_send") {
        return mockSend(args as unknown as RequestSendArgs, command === "request_preview");
      }
      if (command === "update_check") return Promise.resolve(null);
      if (command === "variables_in_scope") {
        return Promise.resolve(variablesFor(args as unknown as RequestSendArgs));
      }
      if (command === "run_start") {
        const onStep = (payload as { onStep: { onmessage: (r: StepResult) => void } }).onStep;
        return mockRun(args as unknown as RunStartArgs, (r) => {
          onStep.onmessage(r);
        });
      }
      if (command === "run_cancel") {
        const id = `run:${(args as unknown as { runId: string }).runId}`;
        const reject = pending.get(id);
        pending.delete(id);
        reject?.({ code: "cancelled", message: "The request was cancelled" });
        return Promise.resolve(reject !== undefined);
      }
      if (command === "example_apply") {
        const { request, example } = args as unknown as { request: Request; example: Example };
        return Promise.resolve(apply(request, example));
      }
      if (command === "example_overlay") {
        const o = args as unknown as { base: Request; draft: Request; id: string; name: string };
        const result = overlay(o.base, o.draft, o.id, o.name);
        return "code" in result ? Promise.reject(result) : Promise.resolve(result);
      }
      const runtimeAnswer = runtimeCommand(command, args);
      if (runtimeAnswer) return runtimeAnswer;
      const spec = specCommand(command, args);
      if (spec) return spec;
      const auth = authCommand(command, args);
      if (auth) return auth;
      const extra = phase4Command(command, args, (send, masked) =>
        resolveFor(send as unknown as RequestSendArgs, masked),
      );
      if (extra) return extra;
      const environment = environmentCommand(
        command,
        args as unknown as Parameters<typeof environmentCommand>[1],
      );
      if (environment) return environment;
      if (command === "request_cancel") {
        const id = (args as unknown as { requestId: string }).requestId;
        const reject = pending.get(id);
        pending.delete(id);
        reject?.({ code: "cancelled", message: "The request was cancelled" });
        return Promise.resolve(reject !== undefined);
      }
      return (
        collectionCommand(command, args) ??
        Promise.reject({ code: "internal", message: `No mock for ${command}` })
      );
    },
    { shouldMockEvents: true },
  );
}
