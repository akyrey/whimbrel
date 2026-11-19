/** Pure helpers for captures and assertions: defaults, edits and a summary of results. */
import type { Assertion, Capture, Checks, StoredRequest } from "../../ipc";

export type AssertionKind = Assertion["check"];
type Change = (draft: StoredRequest) => StoredRequest;

export const ASSERTION_KINDS: [AssertionKind, string][] = [
  ["status", "Status is"],
  ["header", "Header"],
  ["json", "JSON at"],
  ["time", "Time below (ms)"],
];

export function blankAssertion(kind: AssertionKind): Assertion {
  switch (kind) {
    case "status":
      return { check: "status", equals: 200 };
    case "header":
      return { check: "header", name: "", equals: null };
    case "json":
      return { check: "json", path: "$.", equals: null };
    case "time":
      return { check: "time", belowMs: 1000 };
  }
}

export const BLANK_CAPTURE: Capture = {
  var: "",
  from: "body",
  path: "$.",
  regex: null,
  secret: false,
};

const replaceAt = <T>(items: readonly T[], index: number, item: T): T[] =>
  items.map((x, i) => (i === index ? item : x));
const removeAt = <T>(items: readonly T[], index: number): T[] =>
  items.filter((_, i) => i !== index);

export const addCapture: Change = (d) => ({ ...d, captures: [...d.captures, BLANK_CAPTURE] });
export const updateCapture =
  (index: number, change: Partial<Capture>): Change =>
  (d) => {
    const current = d.captures[index];
    return current
      ? { ...d, captures: replaceAt(d.captures, index, { ...current, ...change }) }
      : d;
  };
export const removeCapture =
  (index: number): Change =>
  (d) => ({ ...d, captures: removeAt(d.captures, index) });

export const addAssertion: Change = (d) => ({
  ...d,
  assertions: [...d.assertions, blankAssertion("status")],
});
export const setAssertion =
  (index: number, assertion: Assertion): Change =>
  (d) => ({ ...d, assertions: replaceAt(d.assertions, index, assertion) });
export const removeAssertion =
  (index: number): Change =>
  (d) => ({ ...d, assertions: removeAt(d.assertions, index) });

/** The text of an expected JSON value: JSON when it parses, else the text itself. */
export function parseExpected(text: string): unknown {
  if (text.trim() === "") return null;
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

export const expectedText = (value: unknown): string =>
  value === null || value === undefined
    ? ""
    : typeof value === "string"
      ? value
      : JSON.stringify(value);

/** `2/3 passed`, or `null` when there was nothing to check. */
export function summary(checks: Checks | null): { passed: number; total: number } | null {
  if (!checks) return null;
  const total = checks.assertions.length + checks.captures.length;
  if (total === 0) return null;
  const passed =
    checks.assertions.filter((a) => a.passed).length +
    checks.captures.filter((c) => c.error === null).length;
  return { passed, total };
}
