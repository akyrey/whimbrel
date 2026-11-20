/** `{{` completion and the variables a text uses (FR-2.4). Pure. */
import type { ScopeVariable } from "../../ipc";

export type Completion = { start: number; prefix: string };

const NAME = /^[\w$.-]*$/;

/** The `{{name` being typed at `caret`, if any. */
export function completionAt(text: string, caret: number): Completion | null {
  const before = text.slice(0, caret);
  const start = before.lastIndexOf("{{");
  if (start < 0) return null;
  const prefix = before.slice(start + 2);
  if (!NAME.test(prefix)) return null;
  return { start, prefix };
}

/** Variables whose name starts with the prefix (then contains it), best first. */
export function matching(variables: ScopeVariable[], prefix: string): ScopeVariable[] {
  const p = prefix.toLowerCase();
  const starts = variables.filter((v) => v.name.toLowerCase().startsWith(p));
  const contains = variables.filter(
    (v) => !v.name.toLowerCase().startsWith(p) && v.name.toLowerCase().includes(p),
  );
  return [...starts, ...contains];
}

/** `text` with the completion replaced by `{{name}}`; the caret goes after it. */
export function accept(
  text: string,
  caret: number,
  at: Completion,
  name: string,
): { text: string; caret: number } {
  const after = text.slice(caret);
  const rest = after.startsWith("}}") ? after.slice(2) : after;
  const inserted = `{{${name}}}`;
  return {
    text: `${text.slice(0, at.start)}${inserted}${rest}`,
    caret: at.start + inserted.length,
  };
}

/** A `{{name}}` in a text: where it starts, where it ends (exclusive) and the name. */
export type Token = { start: number; end: number; name: string };

/** Every variable a text uses, in order, with its position. */
export function tokensIn(text: string): Token[] {
  return [...text.matchAll(/\{\{\s*([^{}\s]+)\s*\}\}/g)].flatMap((match) => {
    const name = match[1];
    return name === undefined
      ? []
      : [{ start: match.index, end: match.index + match[0].length, name }];
  });
}

/** The names a text uses, each once, in order. */
export function namesIn(text: string): string[] {
  return [...new Set(tokensIn(text).map((t) => t.name))];
}

const SOURCE_LABEL: Record<ScopeVariable["source"], string> = {
  example: "the example",
  request: "the request",
  runtime: "a capture",
  local: "your local overrides",
  environment: "the environment",
  collection: "the collection",
  dynamic: "generated when sent",
};

/** What hovering a variable says: its value (secrets masked) and where it comes from. */
export function describe(
  name: string,
  variables: ScopeVariable[],
): { text: string; known: boolean } {
  const found = variables.find((v) => v.name === name);
  if (!found) return { text: `${name} is not defined`, known: false };
  const from = SOURCE_LABEL[found.source];
  if (found.source === "dynamic") return { text: `${name}: ${from}`, known: true };
  if (found.secret) {
    return found.set
      ? { text: `${name} = •••••• (secret, from ${from})`, known: true }
      : { text: `${name}: a secret with no value in the keychain`, known: false };
  }
  return { text: `${name} = ${found.value ?? ""} (from ${from})`, known: true };
}

/** What a variable's tooltip shows: the value (secrets masked, `null` when there is none to
 * show) and a note on where it comes from or why it will not resolve. */
export function explain(
  name: string,
  variables: ScopeVariable[],
): { known: boolean; value: string | null; note: string } {
  const found = variables.find((v) => v.name === name);
  if (!found) return { known: false, value: null, note: "Not defined here" };
  const from = SOURCE_LABEL[found.source];
  if (found.source === "dynamic") return { known: true, value: null, note: "Generated when sent" };
  if (found.secret) {
    return found.set
      ? { known: true, value: "••••••", note: `Secret, from ${from}` }
      : { known: false, value: null, note: "Secret with no value in the keychain" };
  }
  return { known: true, value: found.value ?? "", note: `From ${from}` };
}
