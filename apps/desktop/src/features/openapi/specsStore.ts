/** Specs imported into the open collections, and what the spec browser shows. */
import { create } from "zustand";

import { type Api, type StoredSpec, specList, specOpen } from "../../ipc";

export type SpecRef = { root: string; dir: string };

/** What the browser shows: an operation, a schema, or the spec's overview. */
export type SpecFocus =
  | { kind: "overview" }
  | { kind: "operation"; id: string }
  | { kind: "webhook"; id: string }
  | { kind: "schema"; name: string };

export type SpecView = SpecRef & { focus: SpecFocus };

type SpecsState = {
  byRoot: Readonly<Record<string, StoredSpec[]>>;
  /** Loaded models by `root\0dir`. */
  apis: Readonly<Record<string, Api>>;
  view: SpecView | null;
  load: (root: string) => Promise<void>;
  forget: (root: string) => void;
  show: (ref: SpecRef, focus?: SpecFocus) => Promise<void>;
  focus: (focus: SpecFocus) => void;
  close: () => void;
};

export const specKey = (ref: SpecRef): string => `${ref.root}\u0000${ref.dir}`;

/** The spec operation a request file was generated from, if any. */
export function operationOf(
  byRoot: Readonly<Record<string, StoredSpec[]>>,
  root: string,
  path: string,
): { dir: string; id: string } | null {
  for (const spec of byRoot[root] ?? []) {
    const found = Object.entries(spec.operations).find(([, file]) => file === path);
    if (found) return { dir: spec.dir, id: found[0] };
  }
  return null;
}

export const useSpecs = create<SpecsState>()((set, get) => ({
  byRoot: {},
  apis: {},
  view: null,
  load: async (root) => {
    const specs = await specList(root);
    set({ byRoot: { ...get().byRoot, [root]: specs } });
  },
  forget: (root) => {
    set({
      byRoot: Object.fromEntries(Object.entries(get().byRoot).filter(([r]) => r !== root)),
      view: get().view?.root === root ? null : get().view,
    });
  },
  show: async (ref, focus = { kind: "overview" }) => {
    const key = specKey(ref);
    if (!(key in get().apis)) {
      const api = await specOpen(ref.root, ref.dir);
      set({ apis: { ...get().apis, [key]: api } });
    }
    set({ view: { ...ref, focus } });
  },
  focus: (focus) => {
    const view = get().view;
    if (view) set({ view: { ...view, focus } });
  },
  close: () => {
    set({ view: null });
  },
}));

/** Keeps the spec lists in step with the open collections. */
export async function syncSpecs(roots: readonly string[]): Promise<void> {
  const state = useSpecs.getState();
  for (const root of Object.keys(state.byRoot)) {
    if (!roots.includes(root)) state.forget(root);
  }
  await Promise.all(
    roots
      .filter((r) => !(r in useSpecs.getState().byRoot))
      .map((r) => state.load(r).catch(() => undefined)),
  );
}
