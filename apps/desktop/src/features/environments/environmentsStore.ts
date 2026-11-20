import { create } from "zustand";

import { type EnvironmentEntry, environmentList } from "../../ipc";
import { loadJson, saveJson } from "../../lib/storage";

const STORAGE_KEY = "whimbrel.environments";

/** One empty list for every collection without environments: a selector returning a new `[]`
 * each time would make React render forever. */
export const NO_ENVIRONMENTS: readonly EnvironmentEntry[] = [];

type EnvironmentsState = {
  /** By collection root. */
  entries: Record<string, EnvironmentEntry[]>;
  /** The active environment of each collection (`null`: none), remembered across starts. */
  active: Record<string, string | null>;
  /** Grows whenever an environment, its secrets or the collection variables change, so what
   * caches resolved variables knows to fetch them again. */
  revision: number;
  load: (root: string) => Promise<void>;
  setActive: (root: string, id: string | null) => void;
  /** Records a change to variables that `load` would not see (values, secrets). */
  changed: () => void;
};

export const useEnvironments = create<EnvironmentsState>()((set, get) => ({
  entries: {},
  active: loadJson<Record<string, string | null>>(STORAGE_KEY, {}),
  revision: 0,
  load: async (root) => {
    const entries = await environmentList(root);
    const active = get().active[root] ?? null;
    const stillThere = entries.some((e) => e.id === active && e.error === null);
    set({
      entries: { ...get().entries, [root]: entries },
      active: { ...get().active, [root]: stillThere ? active : null },
      revision: get().revision + 1,
    });
  },
  setActive: (root, id) => {
    const active = { ...get().active, [root]: id };
    set({ active });
    saveJson(STORAGE_KEY, active);
  },
  changed: () => {
    set({ revision: get().revision + 1 });
  },
}));

/** The active environment of `root`, if any. */
export function activeEnvironment(root: string | undefined): EnvironmentEntry | null {
  if (root === undefined) return null;
  const { entries, active } = useEnvironments.getState();
  return entries[root]?.find((e) => e.id === active[root]) ?? null;
}

/** The id of a collection's environment picker, so the request bar's chip can focus it. */
export const environmentPickerId = (root: string): string => `environment-picker:${root}`;
