/**
 * Auth for the mocked backend (see mockBackend.ts), following src-tauri/src/auth_apply.rs and
 * oauth.rs: folder and collection auth, inheritance, OAuth 2 tokens kept in memory.
 */
import type {
  AppError,
  Auth,
  CollectionTls,
  OAuthStatus,
  Request,
  RequestSendArgs,
} from "../ipc/generated";

const MASK = "••••••";
const auths = new Map<string, Auth>();
const tls = new Map<string, CollectionTls>();
const tokens = new Map<string, string>();
let issued = 0;

export function resetAuth(): void {
  auths.clear();
  tls.clear();
  tokens.clear();
  issued = 0;
}

const key = (root: string, folder: string) => `${root}|${folder}`;

/** The request's own auth, else its folders' (innermost first), else the collection's. */
export function effectiveAuth(args: RequestSendArgs): Auth {
  const chain: Auth[] = [args.auth ?? { type: "inherit" }];
  if (args.root !== undefined) {
    const folders = (args.path ?? "").split("/").slice(0, -1);
    for (let i = folders.length; i > 0; i -= 1) {
      chain.push(auths.get(key(args.root, folders.slice(0, i).join("/"))) ?? { type: "inherit" });
    }
    chain.push(auths.get(key(args.root, "")) ?? { type: "inherit" });
  }
  return chain.find((a) => a.type !== "inherit") ?? { type: "none" };
}

const tokenKey = (args: RequestSendArgs) =>
  JSON.stringify([args.root ?? null, effectiveAuth(args)]);

/** The request with its auth applied (masked for showing), or `oauth_required`. */
export function applyAuth(
  args: RequestSendArgs,
  request: Request,
  masked: boolean,
): Request | AppError {
  const auth = effectiveAuth(args);
  const secret = (value: string) => (masked ? MASK : value);
  const header = (name: string, value: string): Request =>
    request.headers.some((h) => h.enabled && h.name.toLowerCase() === name.toLowerCase())
      ? request
      : { ...request, headers: [...request.headers, { name, value, enabled: true }] };
  switch (auth.type) {
    case "basic":
      return header("Authorization", `Basic ${secret(btoa(`${auth.username}:${auth.password}`))}`);
    case "bearer":
      return header("Authorization", `Bearer ${secret(auth.token)}`);
    case "api_key":
      return auth.location === "header"
        ? header(auth.name, secret(auth.value))
        : {
            ...request,
            url: `${request.url}${request.url.includes("?") ? "&" : "?"}${auth.name}=${secret(auth.value)}`,
          };
    case "oauth2": {
      const token = tokens.get(tokenKey(args));
      if (token === undefined)
        return {
          code: "oauth_required",
          message: "This request needs an OAuth 2 token: get one first",
        };
      return header("Authorization", `Bearer ${secret(token)}`);
    }
    default:
      return request;
  }
}

function status(args: RequestSendArgs): OAuthStatus {
  const auth = effectiveAuth(args);
  const has = tokens.has(tokenKey(args));
  return {
    configured: auth.type === "oauth2",
    grant: auth.type === "oauth2" ? auth.grant : null,
    hasToken: has,
    expiresAtMs: has ? Date.now() + 3_600_000 : null,
    scope: null,
    canRefresh: has,
  };
}

type Args = Record<string, unknown>;

/** The auth commands, or `null` for any other. */
export function authCommand(command: string, args: Args): Promise<unknown> | null {
  const send = args as unknown as RequestSendArgs;
  switch (command) {
    case "node_auth_read":
      return Promise.resolve(
        auths.get(key(String(args.root), String(args.path))) ?? { type: "inherit" },
      );
    case "node_auth_save":
      auths.set(key(String(args.root), String(args.path)), args.auth as Auth);
      return Promise.resolve(null);
    case "collection_tls_read":
      return Promise.resolve(tls.get(String(args.root)) ?? { caFile: "", clientCertFile: "" });
    case "collection_tls_save":
      tls.set(String(args.root), args.tls as CollectionTls);
      return Promise.resolve(null);
    case "oauth_status":
      return Promise.resolve(status(send));
    case "oauth_authorize":
      issued += 1;
      tokens.set(tokenKey(send), `access-${String(issued)}`);
      return Promise.resolve(status(send));
    case "oauth_forget":
      tokens.delete(tokenKey(send));
      return Promise.resolve(null);
    default:
      return null;
  }
}
