import { create } from "zustand";

export type Side = "collections" | "history";

type SidebarState = {
  side: Side;
  show: (side: Side) => void;
};

/** Which list the sidebar shows; a store so a shortcut can switch it (task 9.5). */
export const useSidebar = create<SidebarState>()((set) => ({
  side: "collections",
  show: (side) => {
    set({ side });
  },
}));
