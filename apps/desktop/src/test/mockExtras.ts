/**
 * Phase 4 parts of the mocked backend (see mockBackend.ts): special response routes, the
 * history, cURL and saving bodies. Follows src-tauri/src/history.rs and commands.rs.
 */
import type { AppError, HistoryEntry, HistorySummary, Request, Response } from "../ipc/generated";

/** A body of `bytes` characters, like a large API response. */
export function bigJson(bytes: number): string {
  const item = '{"id":1,"name":"ada lovelace","tags":["math","engines"]},';
  return `[${item.repeat(Math.max(1, Math.floor(bytes / item.length))).slice(0, -1)}]`;
}

const PNG =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

function make(
  status: number,
  contentType: string,
  body: Partial<Response["body"]>,
  url: string,
): Response {
  const text = body.text ?? null;
  return {
    status,
    reason: status === 200 ? "OK" : "Not Found",
    httpVersion: "HTTP/1.1",
    url,
    headers: [
      { name: "content-type", value: contentType },
      { name: "set-cookie", value: "session=abc; Path=/; HttpOnly" },
    ],
    body: {
      text,
      base64: null,
      size: text?.length ?? 0,
      truncated: false,
      file: null,
      contentType,
      ...body,
    },
    timing: { headersMs: 12, totalMs: 15, dnsMs: null, connectMs: null },
    streamEvents: 0,
  };
}

/** Routes added in Phase 4: `/users` (a JSON array), `/image`, `/binary`, `/huge` (10 MB of
 * JSON), `/spilled` (only the start shown, the rest in a file). `null` for other URLs. */
export function phase4Route(request: Request): Response | null {
  const url = request.url;
  if (url.includes("/users")) {
    return make(
      200,
      "application/json",
      { text: '[{"id":1,"name":"ada"},{"id":2,"name":"grace","admin":true}]' },
      url,
    );
  }
  if (url.includes("/image")) return make(200, "image/png", { base64: PNG, size: 70 }, url);
  if (url.includes("/binary"))
    return make(
      200,
      "application/octet-stream",
      { base64: btoa("PK\u0003\u0004 zip"), size: 9 },
      url,
    );
  if (url.includes("/huge")) {
    const text = bigJson(10 * 1024 * 1024);
    return make(200, "application/json", { text, size: text.length }, url);
  }
  if (url.includes("/spilled")) {
    return make(
      200,
      "text/plain",
      {
        text: "the first megabyte…",
        size: 52_428_800,
        truncated: true,
        file: "/cache/bodies/body-1.bin",
      },
      url,
    );
  }
  return null;
}

let nextId = 0;
const entries: HistoryEntry[] = [];

export function resetHistory(): void {
  entries.length = 0;
}

export function recordHistory(
  request: Request,
  outcome: Response | AppError,
  name: string | undefined,
): void {
  nextId += 1;
  entries.unshift({
    id: nextId,
    sentAtMs: Date.now(),
    collection: null,
    name: name ?? request.url,
    request,
    outcome:
      "code" in outcome
        ? { kind: "failed", code: outcome.code, message: outcome.message }
        : { kind: "completed", response: outcome },
  });
}

const summary = (e: HistoryEntry): HistorySummary => ({
  id: e.id,
  sentAtMs: e.sentAtMs,
  collection: e.collection,
  name: e.name,
  method: e.request.method,
  url: e.request.url,
  status: e.outcome.kind === "completed" ? e.outcome.response.status : null,
  errorCode: e.outcome.kind === "failed" ? e.outcome.code : null,
  durationMs: e.outcome.kind === "completed" ? e.outcome.response.timing.totalMs : null,
});

type Args = Record<string, unknown>;

const quote = (text: string) => `'${text.replaceAll("'", "'\\''")}'`;

function curlOf(request: Request): string {
  const parts = [
    `curl ${request.method === "GET" ? "" : `-X ${request.method} `}${quote(request.url)}`,
  ];
  for (const h of request.headers.filter((x) => x.enabled))
    parts.push(`-H ${quote(`${h.name}: ${h.value}`)}`);
  if (request.body.type === "text") parts.push(`--data-raw ${quote(request.body.text)}`);
  return parts.join(" \\\n  ");
}

/** A small reader for `curl 'url' -H '…' -X … --data-raw '…'`; other options are ignored. */
/** A small JSONPath for the mock: `$`, `.name`, `[*]` and `[n]` (the app uses RFC 9535). */
function selectJson(body: string, path: string): string | { code: string; message: string } {
  const steps = /^\$((?:\.[A-Za-z_]\w*|\[\*\]|\[\d+\])*)$/.exec(path.trim());
  if (!steps) return { code: "invalid_json_path", message: `\`${path}\` is not a JSONPath` };
  let found: unknown[];
  try {
    found = [JSON.parse(body) as unknown];
  } catch {
    return { code: "not_json", message: "The body is not JSON" };
  }
  for (const step of steps[1]?.match(/\.\w+|\[\*\]|\[\d+\]/g) ?? []) {
    found = found.flatMap((value): unknown[] => {
      if (typeof value !== "object" || value === null) return [];
      const record = value as Record<string, unknown>;
      if (step === "[*]") return Object.values(record);
      const key = step.startsWith(".") ? step.slice(1) : step.slice(1, -1);
      return key in record ? [record[key]] : [];
    });
  }
  return JSON.stringify(found, null, 2);
}

function curlImport(text: string) {
  const words = [...text.matchAll(/'([^']*)'|"([^"]*)"|(\S+)/g)].map(
    (m) => m[1] ?? m[2] ?? m[3] ?? "",
  );
  if (words[0] !== "curl") return { code: "invalid_curl", message: "This is not a cURL command" };
  const request: Request = { method: "GET", url: "", headers: [], body: { type: "none" } };
  const ignored: string[] = [];
  for (let i = 1; i < words.length; i += 1) {
    const word = words[i] ?? "";
    const next = () => words[(i += 1)] ?? "";
    if (word === "-H") {
      const [name = "", ...value] = next().split(":");
      request.headers.push({ name: name.trim(), value: value.join(":").trim(), enabled: true });
    } else if (word === "-X") request.method = next();
    else if (word === "--data-raw" || word === "-d") {
      request.body = { type: "text", kind: "json", text: next() };
      if (request.method === "GET") request.method = "POST";
    } else if (word.startsWith("-")) ignored.push(word);
    else request.url = word;
  }
  if (request.url === "") return { code: "invalid_curl", message: "The command has no URL" };
  return {
    request,
    settings: { timeoutMs: null, followRedirects: true, verifyTls: true },
    ignored,
  };
}

/** Phase 4 commands, or `null` for any other. `resolve` gives the request as sent. */
export function phase4Command(
  command: string,
  args: Args,
  resolve: (send: Args, masked: boolean) => Request | AppError,
): Promise<unknown> | null {
  switch (command) {
    case "history_list": {
      const query = String(args.query).toLowerCase();
      const found = entries
        .filter((e) =>
          `${e.name} ${e.request.url} ${e.request.method}`.toLowerCase().includes(query),
        )
        .slice(Number(args.offset), Number(args.offset) + Number(args.limit));
      return Promise.resolve(found.map(summary));
    }
    case "history_get": {
      const entry = entries.find((e) => e.id === args.id);
      return entry
        ? Promise.resolve(entry)
        : Promise.reject({ code: "not_found", message: "That history entry is gone" });
    }
    case "history_clear":
      resetHistory();
      return Promise.resolve(null);
    case "curl_export": {
      const resolved = resolve(args.send as Args, !(args.reveal as boolean));
      return "code" in resolved ? Promise.reject(resolved) : Promise.resolve(curlOf(resolved));
    }
    case "response_select": {
      const selected = selectJson(String(args.body), String(args.path));
      return typeof selected === "string" ? Promise.resolve(selected) : Promise.reject(selected);
    }
    case "curl_import": {
      const imported = curlImport(String(args.text));
      return "code" in imported ? Promise.reject(imported) : Promise.resolve(imported);
    }
    case "response_save":
      return Promise.resolve(null);
    default:
      return null;
  }
}
