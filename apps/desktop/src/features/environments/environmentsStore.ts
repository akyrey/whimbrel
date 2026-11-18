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
  load: (root: string) => Promise<void>;
  setActive: (root: string, id: string | null) => void;
};

export const useEnvironments = create<EnvironmentsState>()((set, get) => ({
  entries: {},
  active: loadJson<Record<string, string | null>>(STORAGE_KEY, {}),
  load: async (root) => {
    const entries = await environmentList(root);
    const active = get().active[root] ?? null;
    const stillThere = entries.some((e) => e.id === active && e.error === null);
    set({
      entries: { ...get().entries, [root]: entries },
      active: { ...get().active, [root]: stillThere ? active : null },
    });
  },
  setActive: (root, id) => {
    const active = { ...get().active, [root]: id };
    set({ active });
    saveJson(STORAGE_KEY, active);
  },
}));

/** The active environment of `root`, if any. */
export function activeEnvironment(root: string | undefined): EnvironmentEntry | null {
  if (root === undefined) return null;
  const { entries, active } = useEnvironments.getState();
  return entries[root]?.find((e) => e.id === active[root]) ?? null;
}
