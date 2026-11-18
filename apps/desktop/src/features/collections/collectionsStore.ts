import { create } from "zustand";

import {
  type CollectionView,
  collectionClose,
  collectionCreate,
  collectionOpen,
  collectionTree,
} from "../../ipc";
import { loadJson, saveJson } from "../../lib/storage";

const STORAGE_KEY = "whimbrel.collections";

type CollectionsState = {
  /** Open collections by root, in the order they were opened. */
  views: Record<string, CollectionView>;
  open: (root: string) => Promise<CollectionView>;
  create: (root: string, name: string) => Promise<CollectionView>;
  close: (root: string) => Promise<void>;
  refresh: (root: string) => Promise<void>;
  /** Reopens the collections open at the last quit; returns the ones that could not be. */
  restore: () => Promise<string[]>;
};

function remember(views: Record<string, CollectionView>) {
  saveJson(STORAGE_KEY, Object.keys(views));
}

export const useCollections = create<CollectionsState>()((set, get) => {
  const put = (view: CollectionView) => {
    const views = { ...get().views, [view.root]: view };
    set({ views });
    remember(views);
    return view;
  };
  return {
    views: {},
    open: async (root) => put(await collectionOpen(root)),
    create: async (root, name) => put(await collectionCreate(root, name)),
    close: async (root) => {
      await collectionClose(root);
      const views = Object.fromEntries(Object.entries(get().views).filter(([r]) => r !== root));
      set({ views });
      remember(views);
    },
    refresh: async (root) => {
      if (!(root in get().views)) return;
      put(await collectionTree(root));
    },
    restore: async () => {
      const roots = loadJson<string[]>(STORAGE_KEY, []);
      const failed: string[] = [];
      for (const root of roots) {
        // A moved or deleted folder is reported, not retried.
        await get()
          .open(root)
          .catch(() => failed.push(root));
      }
      return failed;
    },
  };
});
