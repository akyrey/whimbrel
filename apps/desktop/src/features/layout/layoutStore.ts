/** Panel sizes, kept in localStorage: a per-user convenience, not part of any collection. */
import { create } from "zustand";

import { loadJson, saveJson } from "../../lib/storage";

const STORAGE_KEY = "whimbrel.layout";

export const SIDEBAR_WIDTH = { min: 180, max: 560, initial: 256 } as const;
/** The request's share of the height above the response. */
export const REQUEST_SHARE = { min: 0.15, max: 0.85, initial: 0.45 } as const;

export type Layout = { sidebarWidth: number; requestShare: number };

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

/** Out-of-range or unreadable stored values fall back to something usable. */
export function sanitize(stored: Partial<Layout>): Layout {
  const width = Number(stored.sidebarWidth);
  const share = Number(stored.requestShare);
  return {
    sidebarWidth: Number.isFinite(width)
      ? clamp(width, SIDEBAR_WIDTH.min, SIDEBAR_WIDTH.max)
      : SIDEBAR_WIDTH.initial,
    requestShare: Number.isFinite(share)
      ? clamp(share, REQUEST_SHARE.min, REQUEST_SHARE.max)
      : REQUEST_SHARE.initial,
  };
}

type LayoutState = Layout & {
  resizeSidebar: (width: number) => void;
  resizeRequest: (share: number) => void;
};

export const useLayout = create<LayoutState>()((set, get) => {
  const change = (patch: Partial<Layout>) => {
    const next = sanitize({ ...get(), ...patch });
    set(next);
    saveJson(STORAGE_KEY, next);
  };
  return {
    ...sanitize(loadJson<Partial<Layout>>(STORAGE_KEY, {})),
    resizeSidebar: (sidebarWidth) => {
      change({ sidebarWidth });
    },
    resizeRequest: (requestShare) => {
      change({ requestShare });
    },
  };
});
