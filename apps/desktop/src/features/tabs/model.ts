/** Tabs as data: what is open, what is unsaved, and what a change on disk does to a tab. Pure;
 * every function returns a new tab. */
import type {
  AppError,
  Checks,
  ReadRequest,
  Response,
  StoredRequest,
  StreamEvent,
} from "../../ipc";

/** Stream events (SSE, JSON Lines) arrive while sending and stay with the result. */
export type SendStatus =
  | { kind: "idle" }
  | { kind: "sending"; events: StreamEvent[] }
  | { kind: "done"; response: Response; events: StreamEvent[]; checks: Checks | null }
  | { kind: "failed"; error: AppError; events: StreamEvent[] };

export type FileRef = { root: string; path: string };

export type Tab = {
  id: string;
  /** `null` for a scratch request that has no file yet. */
  file: FileRef | null;
  /** As last read or saved. */
  saved: StoredRequest;
  draft: StoredRequest;
  /** Of the file as last read or saved; `null` for scratch tabs. */
  hash: string | null;
  status: SendStatus;
  /** The file changed on disk while the tab had unsaved edits. */
  conflict: { disk: StoredRequest; hash: string } | null;
  /** The file is gone (deleted or moved outside the app). */
  missing: boolean;
  /** The example sending uses, if one is picked. */
  exampleId: string | null;
};

export const NEW_REQUEST: StoredRequest = {
  name: "New request",
  order: 0,
  request: { method: "GET", url: "", headers: [], body: { type: "none" } },
  examples: [],
  settings: { timeoutMs: null, followRedirects: true, verifyTls: true },
  auth: { type: "inherit" },
  variables: {},
  captures: [],
  assertions: [],
};

export const fileTabId = (file: FileRef): string => `${file.root}\u0000${file.path}`;

export function fileTab(file: FileRef, read: ReadRequest): Tab {
  return {
    id: fileTabId(file),
    file,
    saved: read.stored,
    draft: read.stored,
    hash: read.hash,
    status: { kind: "idle" },
    conflict: null,
    missing: false,
    exampleId: null,
  };
}

/** A tab for a request that is not in a collection (cURL import, history). */
export function looseTab(
  id: string,
  stored: StoredRequest,
  status: SendStatus = { kind: "idle" },
): Tab {
  return { ...scratchTab(id), saved: stored, draft: stored, status };
}

export function scratchTab(id: string): Tab {
  return {
    id,
    file: null,
    saved: NEW_REQUEST,
    draft: NEW_REQUEST,
    hash: null,
    status: { kind: "idle" },
    conflict: null,
    missing: false,
    exampleId: null,
  };
}

/** Structural equality of plain JSON data (requests are). */
function same(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

export const isDirty = (tab: Tab): boolean => tab.file === null || !same(tab.saved, tab.draft);

export const edited = (tab: Tab, draft: StoredRequest): Tab => ({ ...tab, draft });

export const savedAs = (
  tab: Tab,
  hash: string,
  file: FileRef = tab.file ?? { root: "", path: "" },
): Tab => ({
  ...tab,
  id: fileTabId(file),
  file,
  saved: tab.draft,
  hash,
  conflict: null,
  missing: false,
});

/** The file changed on disk: the app's own save (same hash) changes nothing; a saved tab
 * follows the disk; an unsaved one keeps its edits and records the conflict. */
export function changedOnDisk(tab: Tab, read: ReadRequest): Tab {
  if (read.hash === tab.hash) return { ...tab, missing: false };
  if (!isDirty(tab)) {
    return { ...tab, saved: read.stored, draft: read.stored, hash: read.hash, missing: false };
  }
  return { ...tab, conflict: { disk: read.stored, hash: read.hash }, missing: false };
}

/** Keep mine: the next save overwrites the disk. Take theirs: the edits are dropped. */
export function resolved(tab: Tab, keep: "mine" | "theirs"): Tab {
  if (!tab.conflict) return tab;
  if (keep === "mine") return { ...tab, hash: tab.conflict.hash, conflict: null };
  const { disk, hash } = tab.conflict;
  return { ...tab, saved: disk, draft: disk, hash, conflict: null };
}

/** After a rename or move inside the app. */
export function moved(tab: Tab, file: FileRef, name: string): Tab {
  return {
    ...tab,
    id: fileTabId(file),
    file,
    saved: { ...tab.saved, name },
    draft: { ...tab.draft, name },
  };
}

/** Whether `tab` shows `path` of `root`, or something inside the folder `path`. */
export function isUnder(tab: Tab, root: string, path: string): boolean {
  if (!tab.file || tab.file.root !== root) return false;
  return tab.file.path === path || tab.file.path.startsWith(`${path}/`);
}

/** The tab's picked example, if it still exists. */
export function pickedExample(tab: Tab) {
  return tab.draft.examples.find((e) => e.id === tab.exampleId) ?? null;
}

/** The tab `step` away from `active` in `ids`, wrapping at the ends; the first tab when `active`
 * is not open, and `null` without tabs. */
export function neighbourId(
  ids: readonly string[],
  active: string | null,
  step: 1 | -1,
): string | null {
  const index = active === null ? -1 : ids.indexOf(active);
  if (index === -1) return ids[0] ?? null;
  return ids[(index + step + ids.length) % ids.length] ?? null;
}
