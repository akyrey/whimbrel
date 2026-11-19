/** Panel sizes and placement, kept in localStorage: a per-user convenience, not part of any
 * collection. */
import { create } from "zustand";

import { loadJson, saveJson } from "../../lib/storage";

const STORAGE_KEY = "whimbrel.layout";

export const SIDEBAR_WIDTH = { min: 180, max: 560, initial: 256 } as const;
/** The request's share of the space it splits with the response. */
export const REQUEST_SHARE = { min: 0.15, max: 0.85, initial: 0.45 } as const;
export const BESIDE_SHARE_INITIAL = 0.5;

/** Where the response goes: below the request, or beside it. */
export type Split = "below" | "beside";

export type Layout = {
  sidebarWidth: number;
  split: Split;
  /** The request's share of the height when the response is below it. */
  requestShare: number;
  /** The request's share of the width when the response is beside it. */
  besideShare: number;
};

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

const share = (stored: unknown, initial: number) => {
  const n = Number(stored);
  return Number.isFinite(n) ? clamp(n, REQUEST_SHARE.min, REQUEST_SHARE.max) : initial;
};

/** Out-of-range or unreadable stored values fall back to something usable. */
export function sanitize(stored: Partial<Layout>): Layout {
  const width = Number(stored.sidebarWidth);
  return {
    sidebarWidth: Number.isFinite(width)
      ? clamp(width, SIDEBAR_WIDTH.min, SIDEBAR_WIDTH.max)
      : SIDEBAR_WIDTH.initial,
    split: stored.split === "beside" ? "beside" : "below",
    requestShare: share(stored.requestShare, REQUEST_SHARE.initial),
    besideShare: share(stored.besideShare, BESIDE_SHARE_INITIAL),
  };
}

/** The request's share for the current placement. */
export const currentShare = (layout: Layout): number =>
  layout.split === "beside" ? layout.besideShare : layout.requestShare;

/** What a double click on the divider goes back to. */
export const initialShare = (split: Split): number =>
  split === "beside" ? BESIDE_SHARE_INITIAL : REQUEST_SHARE.initial;

type LayoutState = Layout & {
  resizeSidebar: (width: number) => void;
  /** Resizes the request for the current placement. */
  resizeRequest: (share: number) => void;
  placeResponse: (split: Split) => void;
};

export const useLayout = create<LayoutState>()((set, get) => {
  const change = (patch: Partial<Layout>) => {
    const { sidebarWidth, split, requestShare, besideShare } = get();
    const next = sanitize({ sidebarWidth, split, requestShare, besideShare, ...patch });
    set(next);
    saveJson(STORAGE_KEY, next);
  };
  return {
    ...sanitize(loadJson<Partial<Layout>>(STORAGE_KEY, {})),
    resizeSidebar: (sidebarWidth) => {
      change({ sidebarWidth });
    },
    resizeRequest: (to) => {
      change(get().split === "beside" ? { besideShare: to } : { requestShare: to });
    },
    placeResponse: (split) => {
      change({ split });
    },
  };
});
