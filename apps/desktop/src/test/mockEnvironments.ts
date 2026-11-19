/**
 * Environments, variables, secrets and examples for the mocked backend (see mockBackend.ts),
 * following src-tauri/src/environments.rs and resolve.rs: values resolve example → .local →
 * environment (secrets from the "keychain") → collection; previews mask secrets; saving text
 * that holds a secret value is refused.
 */
import { runtimeScope } from "./mockChecks";
import type {
  AppError,
  Environment,
  EnvironmentEntry,
  EnvironmentView,
  Example,
  Header,
  Request,
  RequestSendArgs,
} from "../ipc/generated";

type Stored = { environment: Environment; local: Record<string, string>; hash: string };

const MASK = "••••••";
const MIN_GUARDED_SECRET = 4;

let version = 0;
const nextHash = () => `e${String((version += 1))}`;
const environments = new Map<string, Map<string, Stored>>();
const collectionVariables = new Map<string, Record<string, string>>();
const secrets = new Map<string, string>();

const account = (root: string, env: string, name: string) => `${root}:${env}:${name}`;

export function seedEnvironments(root: string): void {
  environments.clear();
  collectionVariables.clear();
  secrets.clear();
  environments.set(
    root,
    new Map([
      [
        "local",
        {
          environment: {
            name: "Local",
            protected: false,
            variables: { base: "http://localhost" },
            secrets: [],
          },
          local: {},
          hash: nextHash(),
        },
      ],
      [
        "production",
        {
          environment: {
            name: "Production",
            protected: true,
            variables: { base: "https://api.example.com" },
            secrets: [{ name: "apiKey", description: "From the vault" }],
          },
          local: {},
          hash: nextHash(),
        },
      ],
    ]),
  );
}

const fail = (code: string, message: string, detail?: unknown): Promise<never> =>
  Promise.reject({ code, message, ...(detail === undefined ? {} : { detail }) } satisfies AppError);

const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "environment";

function entries(root: string): EnvironmentEntry[] {
  return [...(environments.get(root) ?? new Map<string, Stored>())]
    .map(([id, s]) => ({
      id,
      name: s.environment.name,
      protected: s.environment.protected,
      error: null,
    }))
    .sort((a, b) => a.id.localeCompare(b.id));
}

function view(root: string, id: string, s: Stored): EnvironmentView {
  return {
    id,
    environment: s.environment,
    local: s.local,
    hash: s.hash,
    secrets: s.environment.secrets.map((x) => ({
      name: x.name,
      set: secrets.has(account(root, id, x.name)),
    })),
  };
}

type Args = Record<string, unknown> & { root: string; id: string };

/** The environment commands, or `null` for any other command. */
export function environmentCommand(command: string, a: Args): Promise<unknown> | null {
  const envs = environments.get(a.root) ?? new Map<string, Stored>();
  environments.set(a.root, envs);
  const current = envs.get(a.id);
  switch (command) {
    case "environment_list":
      return Promise.resolve(entries(a.root));
    case "environment_read":
      return current
        ? Promise.resolve(view(a.root, a.id, current))
        : fail("io", "No such environment");
    case "environment_create": {
      const environment = a.environment as Environment;
      let id = slug(environment.name);
      for (let n = 2; envs.has(id); n += 1) id = `${slug(environment.name)}-${String(n)}`;
      envs.set(id, { environment, local: {}, hash: nextHash() });
      return Promise.resolve(id);
    }
    case "environment_save": {
      if (!current) return fail("io", "No such environment");
      if (current.hash !== a.hash)
        return fail("conflict", "The file changed on disk since it was opened");
      const saved = { ...current, environment: a.environment as Environment, hash: nextHash() };
      envs.set(a.id, saved);
      return Promise.resolve(saved.hash);
    }
    case "environment_delete":
      envs.delete(a.id);
      return Promise.resolve(null);
    case "variables_save": {
      const variables = a.variables as Record<string, string>;
      if (typeof a.id === "string" && current) envs.set(a.id, { ...current, local: variables });
      else collectionVariables.set(a.root, variables);
      return Promise.resolve(null);
    }
    case "collection_variables_read":
      return Promise.resolve(collectionVariables.get(a.root) ?? {});
    case "secret_set": {
      const env = envs.get(a.environment as string);
      if (!env?.environment.secrets.some((s) => s.name === a.name))
        return fail("not_found", "No such secret");
      secrets.set(account(a.root, a.environment as string, a.name as string), a.value as string);
      return Promise.resolve(null);
    }
    case "secret_clear":
      secrets.delete(account(a.root, a.environment as string, a.name as string));
      return Promise.resolve(null);
    default:
      return null;
  }
}

type Value = { plain: string } | { secret: string | null };

function scopes(args: RequestSendArgs): Record<string, Value>[] {
  const out: Record<string, Value>[] = [];
  const plain = (vars: Record<string, string>) =>
    Object.fromEntries(Object.entries(vars).map(([k, v]) => [k, { plain: v }]));
  if (args.example) out.push(plain(args.example.variables));
  if (args.variables) out.push(plain(args.variables));
  out.push(runtimeScope(args.root));
  if (args.root === undefined) return out;
  const env =
    args.environment === undefined ? undefined : environments.get(args.root)?.get(args.environment);
  if (env && args.environment !== undefined) {
    out.push(plain(env.local));
    const values: Record<string, Value> = plain(env.environment.variables);
    for (const s of env.environment.secrets) {
      values[s.name] = {
        secret: secrets.get(account(args.root, args.environment, s.name)) ?? null,
      };
    }
    out.push(values);
  }
  out.push(plain(collectionVariables.get(args.root) ?? {}));
  return out;
}

const VARIABLE = /\{\{\s*(\$?[A-Za-z_][A-Za-z0-9_.-]*)\s*\}\}/g;

type Resolution = { request: Request; secrets: string[]; undefined: string[]; unset: string[] };

function resolveRequest(request: Request, layers: Record<string, Value>[]): Resolution {
  const out: Resolution = { request, secrets: [], undefined: [], unset: [] };
  const once = (list: string[], item: string) => {
    if (!list.includes(item)) list.push(item);
  };
  const run = (text: string) =>
    text.replace(VARIABLE, (whole, name: string) => {
      if (name === "$uuid") return "00000000-0000-4000-8000-000000000000";
      const value = layers.map((l) => l[name]).find((v) => v !== undefined);
      if (value === undefined) {
        once(out.undefined, name);
        return whole;
      }
      if ("plain" in value) return value.plain;
      if (value.secret === null) {
        once(out.unset, name);
        return whole;
      }
      once(out.secrets, value.secret);
      return value.secret;
    });
  out.request = {
    method: request.method,
    url: run(request.url),
    headers: request.headers.map((h) =>
      h.enabled ? { ...h, name: run(h.name), value: run(h.value) } : h,
    ),
    body:
      request.body.type === "text"
        ? { ...request.body, text: run(request.body.text) }
        : request.body,
  };
  return out;
}

const mask = (text: string, values: string[]) =>
  values.filter((v) => v !== "").reduce((acc, v) => acc.split(v).join(MASK), text);

/** The request to send (or preview, `masked`), or the `unresolved` error. */
export function resolveFor(args: RequestSendArgs, masked: boolean): Request | AppError {
  const base = args.example ? apply(args.request, args.example) : args.request;
  const done = resolveRequest(base, scopes(args));
  if (done.undefined.length > 0 || done.unset.length > 0) {
    const parts = [
      done.undefined.length > 0 ? `Not defined: ${done.undefined.join(", ")}` : "",
      done.unset.length > 0 ? `No value in the keychain for: ${done.unset.join(", ")}` : "",
    ].filter(Boolean);
    return {
      code: "unresolved",
      message: parts.join(". "),
      detail: { undefined: done.undefined, unsetSecrets: done.unset },
    };
  }
  if (!masked) return done.request;
  const m = (text: string) => mask(text, done.secrets);
  const r = done.request;
  return {
    ...r,
    url: m(r.url),
    headers: r.headers.map((h) => ({ ...h, value: m(h.value) })),
    body: r.body.type === "text" ? { ...r.body, text: m(r.body.text) } : r.body,
  };
}

/** `contains_secret` when `text` holds a secret value of the collection. */
export function guard(root: string, text: string): AppError | null {
  for (const [key, value] of secrets) {
    if (key.startsWith(`${root}:`) && value.length >= MIN_GUARDED_SECRET && text.includes(value)) {
      const name = key.split(":").pop() ?? "";
      return {
        code: "contains_secret",
        message: `This holds the value of the secret ${name}. Use {{${name}}} instead.`,
      };
    }
  }
  return null;
}

function splitUrl(url: string) {
  const [rest = "", fragment] = url.split("#", 2);
  const [head = "", query = ""] = rest.split("?", 2);
  const pairs = query
    .split("&")
    .filter(Boolean)
    .map((p) => {
      const [name = "", value = ""] = p.split("=", 2);
      return [name, value] as [string, string];
    });
  return { head, pairs, fragment };
}

function joinUrl(head: string, pairs: [string, string][], fragment: string | undefined) {
  const query = pairs.map(([n, v]) => `${n}=${v}`).join("&");
  return `${head}${query ? `?${query}` : ""}${fragment === undefined ? "" : `#${fragment}`}`;
}

export function apply(request: Request, example: Example): Request {
  const { head, pairs, fragment } = splitUrl(request.url);
  let query = pairs;
  for (const p of example.query) {
    if (!p.enabled) query = query.filter(([n]) => n !== p.name);
    else if (query.some(([n]) => n === p.name))
      query = query.map(([n, v]) => [n, n === p.name ? p.value : v]);
    else query = [...query, [p.name, p.value]];
  }
  let headers: Header[] = request.headers;
  for (const h of example.headers) {
    const same = (x: Header) => x.name.toLowerCase() === h.name.toLowerCase();
    if (!h.enabled) headers = headers.map((x) => (same(x) ? { ...x, enabled: false } : x));
    else if (headers.some(same))
      headers = headers.map((x) => (same(x) ? { ...x, value: h.value, enabled: true } : x));
    else headers = [...headers, h];
  }
  return {
    ...request,
    url: example.query.length > 0 ? joinUrl(head, query, fragment) : request.url,
    headers,
    body: example.body ?? request.body,
  };
}

export function overlay(
  base: Request,
  draft: Request,
  id: string,
  name: string,
): Example | AppError {
  const from = splitUrl(base.url);
  const to = splitUrl(draft.url);
  if (base.method !== draft.method || from.head !== to.head || from.fragment !== to.fragment) {
    return {
      code: "invalid_example",
      message:
        "An example keeps the request's method and path; change those in the request, or use a variable",
    };
  }
  const has = (list: [string, string][], [n, v]: [string, string]) =>
    list.some(([a, b]) => a === n && b === v);
  return {
    id,
    name,
    order: 0,
    variables: {},
    query: [
      ...to.pairs
        .filter((p) => !has(from.pairs, p))
        .map(([n, v]) => ({ name: n, value: v, enabled: true })),
      ...from.pairs
        .filter(([n]) => !to.pairs.some(([d]) => d === n))
        .map(([n, v]) => ({ name: n, value: v, enabled: false })),
    ],
    headers: draft.headers.filter(
      (h) =>
        !base.headers.some(
          (b) => b.name === h.name && b.value === h.value && b.enabled === h.enabled,
        ),
    ),
    body: JSON.stringify(base.body) === JSON.stringify(draft.body) ? null : draft.body,
  };
}
