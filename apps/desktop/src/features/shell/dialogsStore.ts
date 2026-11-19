/** Which app-wide dialog is open, so the sidebar, the palette and shortcuts open the same ones. */
import { create } from "zustand";

export type AppDialog = "curl" | "openapi" | "interop" | "settings";

type DialogsState = {
  open: AppDialog | null;
  show: (dialog: AppDialog) => void;
  close: () => void;
};

export const useDialogs = create<DialogsState>()((set) => ({
  open: null,
  show: (dialog) => {
    set({ open: dialog });
  },
  close: () => {
    set({ open: null });
  },
}));
