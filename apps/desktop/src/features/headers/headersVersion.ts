import { create } from "zustand";

/** Bumped when a folder's or the collection's headers are saved, so views of what a request
 * inherits read them again. */
export const useHeadersVersion = create<{ version: number; bump: () => void }>()((set, get) => ({
  version: 0,
  bump: () => {
    set({ version: get().version + 1 });
  },
}));
