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
import { phase4Command, phase4Route, recordHistory, resetHistory } from "./mockExtras";
import {
  apply,
  environmentCommand,
  guard,
  overlay,
  resolveFor,
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
  StoredRequest,
} from "../ipc/generated";

type File = { stored: StoredRequest; hash: string };
type Fs = { id: string; name: string; files: Map<string, File>; folders: Map<string, string> };

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
});

function seed(): void {
  collections.clear();
  collections.set(DEMO_ROOT, {
    id: "demo-id",
    name: "Demo API",
    folders: new Map([["users", "Users"]]),
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
    .map(([path, name]) => ({ kind: "folder", path, name, order: 0, children: nodes(fs, path) }));
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
  return [...files, ...folders];
}

const view = (root: string, fs: Fs): CollectionView => ({
  root,
  collection: { id: fs.id, name: fs.name, children: nodes(fs, "") },
});

function collection(root: string): Fs | null {
  return collections.get(root) ?? null;
}

type Args = Record<string, unknown> & { root: string; path: string; name: string };

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
      if (!file) return fail("io", "No such file");
      fs.files.delete(a.path);
      const path = unique(fs, parentOf(a.path), a.name, ".req.toml");
      fs.files.set(path, { stored: { ...file.stored, name: a.name }, hash: nextHash() });
      return Promise.resolve(path);
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
  if (request.url.includes("/echo")) {
    const sent = {
      method: request.method,
      headers: request.headers.filter((h) => h.enabled).map((h) => [h.name, h.value]),
      body: request.body.type === "text" ? request.body.text : "",
    };
    return { ...response(200, "OK", "application/json", JSON.stringify(sent)), url: request.url };
  }
  return { ...response(404, "Not Found", "text/plain", "no such route"), url: request.url };
}

const isError = (value: Response | AppError): value is AppError => "code" in value;

export function installMockBackend(): void {
  seed();
  seedEnvironments(DEMO_ROOT);
  resetHistory();
  resetAuth();
  const pending = new Map<string, (error: AppError) => void>();
  mockIPC(
    (command, payload) => {
      const args = (payload as { args?: Args } | undefined)?.args ?? ({} as Args);
      if (command === "plugin:dialog|open") return Promise.resolve(DEMO_ROOT);
      if (command === "plugin:dialog|save") return Promise.resolve("/tmp/saved-body");
      if (command === "request_preview" || command === "request_send") {
        const sendArgs = args as unknown as RequestSendArgs;
        const variables = resolveFor(sendArgs, command === "request_preview");
        if ("code" in variables) return Promise.reject(variables);
        const resolved = applyAuth(sendArgs, variables, command === "request_preview");
        if ("code" in resolved) return Promise.reject(resolved);
        if (command === "request_preview") return Promise.resolve(resolved);
        const { requestId } = sendArgs;
        const request = resolved;
        if (request.url.includes("/slow")) {
          return new Promise((_, reject: (error: AppError) => void) => {
            pending.set(requestId, reject);
          });
        }
        const result = answer(request);
        recordHistory(resolveFor(sendArgs, true) as Request, result, sendArgs.name);
        return isError(result) ? Promise.reject(result) : Promise.resolve(result);
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
