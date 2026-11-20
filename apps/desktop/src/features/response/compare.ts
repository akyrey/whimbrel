/** Comparing a response with the one before it (task 10.3). Pure. */
import { diffLines } from "diff";

import type { Response, ResponseHeader } from "../../ipc";
import { formatSize, showBody } from "./format";

export type DiffLine = { kind: "same" | "added" | "removed"; text: string };

export type ShownLine = DiffLine | { kind: "skipped"; count: number };

export type HeaderChange = { name: string; before: string | null; after: string | null };

/** Bodies larger than this are not diffed line by line: it could take seconds. */
export const COMPARE_LIMIT = 512 * 1024;

/** The body as compared: pretty JSON when it is JSON, a size note when it is binary. */
export function comparableBody(response: Response): string {
  const shown = showBody(response.body);
  if (shown.kind === "code") return shown.text;
  if (shown.kind === "binary") return `(binary, ${formatSize(shown.size)})`;
  return "";
}

/** jsdiff treats a last line without a newline as different from the same line with one. */
const terminated = (text: string) => (text === "" || text.endsWith("\n") ? text : `${text}\n`);

/** Line by line, what stayed, what went and what came. */
export function diffBodies(before: string, after: string): DiffLine[] {
  return diffLines(terminated(before), terminated(after)).flatMap((change) => {
    const kind = change.added ? "added" : change.removed ? "removed" : "same";
    return change.value
      .slice(0, -1)
      .split("\n")
      .map((text) => ({ kind, text }));
  });
}

/** The changes with `context` unchanged lines around each; longer unchanged runs are folded. */
export function withContext(lines: DiffLine[], context: number): ShownLine[] {
  const changed = lines.flatMap((line, i) => (line.kind === "same" ? [] : [i]));
  const near = (i: number) => changed.some((c) => Math.abs(c - i) <= context);
  return lines.reduce<ShownLine[]>((shown, line, i) => {
    if (line.kind !== "same" || near(i)) return [...shown, line];
    const last = shown.at(-1);
    return last?.kind === "skipped"
      ? [...shown.slice(0, -1), { kind: "skipped", count: last.count + 1 }]
      : [...shown, { kind: "skipped", count: 1 }];
  }, []);
}

/** Each header's values by lower-case name, repeated headers joined. */
function byName(headers: ResponseHeader[]): Map<string, string> {
  return headers.reduce((map, h) => {
    const name = h.name.toLowerCase();
    const seen = map.get(name);
    return new Map(map).set(name, seen === undefined ? h.value : `${seen}, ${h.value}`);
  }, new Map<string, string>());
}

/** Headers added, removed or changed, by name. */
export function headerChanges(before: ResponseHeader[], after: ResponseHeader[]): HeaderChange[] {
  const old = byName(before);
  const now = byName(after);
  const names = [...new Set([...old.keys(), ...now.keys()])].sort();
  return names.flatMap((name) => {
    const was = old.get(name) ?? null;
    const is = now.get(name) ?? null;
    return was === is ? [] : [{ name, before: was, after: is }];
  });
}
