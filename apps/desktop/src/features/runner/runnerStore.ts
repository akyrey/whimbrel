/** Which folder the runner dialog is open for. */
import { create } from "zustand";

export type RunTarget = { root: string; folder: string; name: string };

type RunnerState = {
  target: RunTarget | null;
  open: (target: RunTarget) => void;
  close: () => void;
};

export const useRunner = create<RunnerState>()((set) => ({
  target: null,
  open: (target) => {
    set({ target });
  },
  close: () => {
    set({ target: null });
  },
}));
