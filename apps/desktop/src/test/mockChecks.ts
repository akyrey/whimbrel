/**
 * Captures, assertions and runtime variables of the mocked backend (see mockBackend.ts),
 * following src-tauri/src/checks.rs and runtime.rs. JSONPath here is the simple subset the
 * tests use: `$.a.b[0]`.
 */
import type {
  Assertion,
  Capture,
  CheckResult,
  Checks,
  RequestSendArgs,
  Response,
  RuntimeVariable,
} from "../ipc/generated";

type Stored = { value: string; secret: boolean };
const LOOSE = "";
let runtime = new Map<string, Map<string, Stored>>();

export function resetRuntime(): void {
  runtime = new Map();
}

const keyOf = (root: string | undefined) => root ?? LOOSE;

/** The runtime scope as the mock's resolver reads it. */
export function runtimeScope(
  root: string | undefined,
): Record<string, { plain: string } | { secret: string | null }> {
  const out: Record<string, { plain: string } | { secret: string | null }> = {};
  for (const [name, v] of runtime.get(keyOf(root)) ?? []) {
    out[name] = v.secret ? { secret: v.value } : { plain: v.value };
  }
  return out;
}

function query(body: string | null, path: string): { found: unknown } | null {
  let value: unknown;
  try {
    value = JSON.parse(body ?? "");
  } catch {
    return null;
  }
  const steps = path.replace(/^\$\.?/, "").match(/[^.[\]]+/g) ?? [];
  for (const step of steps) {
    if (typeof value !== "object" || value === null) return { found: undefined };
    value = (value as Record<string, unknown>)[step];
  }
  return { found: value };
}

const headerOf = (response: Response, name: string) =>
  response.headers.find((h) => h.name.toLowerCase() === name.toLowerCase())?.value;

function capture(c: Capture, response: Response): string | Error {
  if (c.from === "header")
    return headerOf(response, c.path) ?? new Error(`Nothing found at header ${c.path}`);
  const result = query(response.body.text, c.path);
  if (result === null) return new Error("The body is not JSON");
  if (result.found === undefined) return new Error(`Nothing found at ${c.path}`);
  return typeof result.found === "string" ? result.found : JSON.stringify(result.found);
}

function assertOne(a: Assertion, response: Response): CheckResult {
  switch (a.check) {
    case "status":
      return response.status === a.equals
        ? { passed: true, message: `status ${String(response.status)}` }
        : {
            passed: false,
            message: `status ${String(response.status)}, expected ${String(a.equals)}`,
          };
    case "header": {
      const v = headerOf(response, a.name);
      if (v === undefined) return { passed: false, message: `header ${a.name} is missing` };
      const ok = a.equals == null || v === a.equals;
      return { passed: ok, message: `header ${a.name} is ${v}` };
    }
    case "json": {
      const r = query(response.body.text, a.path);
      if (r?.found === undefined) return { passed: false, message: `${a.path} is missing` };
      const ok = a.equals == null || JSON.stringify(r.found) === JSON.stringify(a.equals);
      return { passed: ok, message: `${a.path} is ${JSON.stringify(r.found)}` };
    }
    case "time":
      return {
        passed: response.timing.totalMs < a.belowMs,
        message: `${String(response.timing.totalMs)} ms`,
      };
  }
}

export function checksFor(args: RequestSendArgs, response: Response): Checks {
  const key = keyOf(args.root);
  const captures = (args.captures ?? []).map((c) => {
    const value = capture(c, response);
    if (value instanceof Error) {
      return { var: c.var, secret: c.secret, value: null, error: value.message, saved: false };
    }
    const vars = runtime.get(key) ?? new Map<string, Stored>();
    vars.set(c.var, { value, secret: c.secret });
    runtime.set(key, vars);
    return {
      var: c.var,
      secret: c.secret,
      value: c.secret ? null : value,
      error: null,
      saved: false,
    };
  });
  return { assertions: (args.assertions ?? []).map((a) => assertOne(a, response)), captures };
}

export function runtimeCommand(command: string, args: { root?: string }): Promise<unknown> | null {
  if (command === "runtime_list") {
    const list: RuntimeVariable[] = [...(runtime.get(keyOf(args.root)) ?? [])].map(([name, v]) => ({
      name,
      value: v.secret ? null : v.value,
      secret: v.secret,
    }));
    return Promise.resolve(list);
  }
  if (command === "runtime_clear") {
    runtime.delete(keyOf(args.root));
    return Promise.resolve(null);
  }
  return null;
}
