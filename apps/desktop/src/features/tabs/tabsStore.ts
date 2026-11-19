import { create } from "zustand";

import { useCollections } from "../collections/collectionsStore";
import { activeEnvironment } from "../environments/environmentsStore";
import {
  type RequestSendArgs,
  type StoredRequest,
  type StreamEvent,
  requestCancel,
  requestCreate,
  requestRead,
  requestSave,
  requestSend,
  toAppError,
} from "../../ipc";
import { loadJson, saveJson } from "../../lib/storage";
import {
  type FileRef,
  type SendStatus,
  type Tab,
  changedOnDisk,
  fileTab,
  fileTabId,
  isUnder,
  looseTab,
  moved,
  pickedExample,
  resolved,
  savedAs,
  scratchTab,
} from "./model";

const STORAGE_KEY = "whimbrel.tabs";

type Stored = { files: FileRef[]; active: string | null };

type TabsState = {
  tabs: Tab[];
  activeId: string | null;
  openFile: (file: FileRef) => Promise<void>;
  newScratch: () => void;
  activate: (id: string) => void;
  close: (id: string) => void;
  edit: (id: string, change: (draft: StoredRequest) => StoredRequest) => void;
  /** Saves to its file; a scratch tab is created in `folder` of `root`. */
  save: (id: string, target?: FileRef) => Promise<void>;
  resolve: (id: string, keep: "mine" | "theirs") => void;
  send: (id: string) => Promise<void>;
  /** Opens a request that has no file (cURL import, history) in a new tab. */
  openLoose: (stored: StoredRequest, status?: SendStatus) => void;
  /** Picks the example sending uses (`null`: the request as it is). */
  pickExample: (id: string, exampleId: string | null) => void;
  cancel: (id: string) => Promise<void>;
  changedOnDisk: (root: string, paths: string[]) => Promise<void>;
  renamed: (root: string, from: string, to: string, name: string) => void;
  removed: (root: string, path: string) => void;
  restore: () => Promise<void>;
};

let scratchCount = 0;

/** The collection a tab's variables come from: its own, or the first open one for a scratch
 * request. */
export function rootOf(tab: Tab): string | undefined {
  return tab.file?.root ?? Object.keys(useCollections.getState().views)[0];
}

/** What sending (or previewing) the tab needs. */
export function sendArgs(tab: Tab): RequestSendArgs {
  const root = rootOf(tab);
  const environment = activeEnvironment(root)?.id;
  const example = pickedExample(tab);
  return {
    requestId: tab.id,
    request: tab.draft.request,
    name: tab.draft.name,
    settings: tab.draft.settings,
    auth: tab.draft.auth,
    variables: tab.draft.variables,
    captures: tab.draft.captures,
    assertions: tab.draft.assertions,
    ...(tab.file === null ? {} : { path: tab.file.path }),
    ...(root === undefined ? {} : { root }),
    ...(environment === undefined ? {} : { environment }),
    ...(example === null ? {} : { example }),
  };
}

export const useTabs = create<TabsState>()((set, get) => {
  const replace = (id: string, change: (tab: Tab) => Tab) => {
    set({ tabs: get().tabs.map((t) => (t.id === id ? change(t) : t)) });
  };
  const remember = () => {
    const files = get().tabs.flatMap((t) => (t.file ? [t.file] : []));
    saveJson(STORAGE_KEY, { files, active: get().activeId } satisfies Stored);
  };
  const find = (id: string) => get().tabs.find((t) => t.id === id);

  return {
    tabs: [],
    activeId: null,
    openFile: async (file) => {
      const id = fileTabId(file);
      if (!find(id)) {
        const tab = fileTab(file, await requestRead(file.root, file.path));
        // Another open of the same file may have finished while this one read it.
        if (!find(id)) set({ tabs: [...get().tabs, tab] });
      }
      set({ activeId: id });
      remember();
    },
    newScratch: () => {
      scratchCount += 1;
      const tab = scratchTab(`scratch-${String(scratchCount)}`);
      set({ tabs: [...get().tabs, tab], activeId: tab.id });
    },
    activate: (id) => {
      set({ activeId: id });
      remember();
    },
    close: (id) => {
      const tabs = get().tabs;
      const index = tabs.findIndex((t) => t.id === id);
      const rest = tabs.filter((t) => t.id !== id);
      const neighbour = rest[Math.min(index, rest.length - 1)]?.id ?? null;
      set({ tabs: rest, activeId: get().activeId === id ? neighbour : get().activeId });
      remember();
    },
    edit: (id, change) => {
      replace(id, (t) => ({ ...t, draft: change(t.draft) }));
    },
    save: async (id, target) => {
      const tab = find(id);
      if (!tab) return;
      // A file that is gone is written again in its folder.
      const destination =
        tab.file && tab.missing
          ? { root: tab.file.root, path: tab.file.path.split("/").slice(0, -1).join("/") }
          : target;
      if (tab.file && !tab.missing && tab.hash !== null) {
        const hash = await requestSave({ ...tab.file, stored: tab.draft, hash: tab.hash });
        replace(id, (t) => savedAs(t, hash));
        remember();
        return;
      }
      if (destination) {
        const path = await requestCreate({
          root: destination.root,
          folder: destination.path,
          stored: tab.draft,
        });
        const read = await requestRead(destination.root, path);
        const file = { root: destination.root, path };
        set({
          tabs: get().tabs.map((t) =>
            t.id === id ? savedAs({ ...t, draft: read.stored }, read.hash, file) : t,
          ),
          activeId: get().activeId === id ? fileTabId(file) : get().activeId,
        });
      }
      remember();
    },
    resolve: (id, keep) => {
      replace(id, (t) => resolved(t, keep));
    },
    send: async (id) => {
      const tab = find(id);
      if (!tab || tab.status.kind === "sending") return;
      replace(id, (t) => ({ ...t, status: { kind: "sending", events: [] } }));
      const events = () => {
        const status = find(id)?.status;
        return status && status.kind !== "idle" ? status.events : [];
      };
      const onEvent = (event: StreamEvent) => {
        replace(id, (t) =>
          t.status.kind === "sending"
            ? { ...t, status: { kind: "sending", events: [...t.status.events, event] } }
            : t,
        );
      };
      try {
        const { response, checks } = await requestSend(sendArgs(tab), onEvent);
        replace(id, (t) => ({
          ...t,
          status: { kind: "done", response, events: events(), checks },
        }));
      } catch (caught) {
        replace(id, (t) => ({
          ...t,
          status: { kind: "failed", error: toAppError(caught), events: events() },
        }));
      }
    },
    openLoose: (stored, status) => {
      scratchCount += 1;
      const tab = looseTab(`scratch-${String(scratchCount)}`, stored, status);
      set({ tabs: [...get().tabs, tab], activeId: tab.id });
    },
    pickExample: (id, exampleId) => {
      replace(id, (t) => ({ ...t, exampleId }));
    },
    cancel: async (id) => {
      if (find(id)?.status.kind !== "sending") return;
      // The pending send rejects with `cancelled`, which ends the sending state.
      await requestCancel(id).catch(() => false);
    },
    changedOnDisk: async (root, paths) => {
      const affected = get().tabs.filter(
        (t) => t.file?.root === root && paths.includes(t.file.path),
      );
      for (const tab of affected) {
        const file = tab.file;
        if (!file) continue;
        try {
          const read = await requestRead(file.root, file.path);
          replace(tab.id, (t) => changedOnDisk(t, read));
        } catch {
          replace(tab.id, (t) => ({ ...t, missing: true }));
        }
      }
    },
    renamed: (root, from, to, name) => {
      set({
        tabs: get().tabs.map((t) =>
          t.file?.root === root && t.file.path === from ? moved(t, { root, path: to }, name) : t,
        ),
        activeId:
          get().activeId === fileTabId({ root, path: from })
            ? fileTabId({ root, path: to })
            : get().activeId,
      });
      remember();
    },
    removed: (root, path) => {
      const gone = get().tabs.filter((t) => isUnder(t, root, path));
      gone.forEach((t) => {
        get().close(t.id);
      });
    },
    restore: async () => {
      const stored = loadJson<Stored>(STORAGE_KEY, { files: [], active: null });
      for (const file of stored.files) {
        // A file that is gone is simply not reopened.
        await get()
          .openFile(file)
          .catch(() => undefined);
      }
      const active = stored.active;
      if (active && find(active)) set({ activeId: active });
    },
  };
});

/** The active tab, or `undefined`. */
export function useActiveTab(): Tab | undefined {
  return useTabs((s) => s.tabs.find((t) => t.id === s.activeId));
}
