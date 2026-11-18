import { create } from "zustand";

/** Bumped when a folder's or the collection's auth is saved: what a request inherits may have
 * changed, so views of it read it again. */
export const useAuthVersion = create<{ version: number; bump: () => void }>()((set, get) => ({
  version: 0,
  bump: () => {
    set({ version: get().version + 1 });
  },
}));
