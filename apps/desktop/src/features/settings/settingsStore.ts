/** The user's settings (FR-9.1), kept in localStorage. */
import type { ThemeSetting } from "@roost/ui";
import { create } from "zustand";

import { loadJson, saveJson } from "../../lib/storage";

const STORAGE_KEY = "whimbrel.settings";
export const FONT_SIZES = [11, 12, 13, 14, 15, 16, 18, 20] as const;

export type Settings = {
  theme: ThemeSetting;
  /** Editor font size in pixels. */
  fontSize: number;
  /** For requests without their own timeout; `null`: the engine's (30 s). */
  timeoutMs: number | null;
};

export const DEFAULT_SETTINGS: Settings = { theme: "system", fontSize: 13, timeoutMs: null };

type SettingsState = Settings & { change: (patch: Partial<Settings>) => void };

export const useSettings = create<SettingsState>()((set, get) => ({
  ...DEFAULT_SETTINGS,
  ...loadJson<Partial<Settings>>(STORAGE_KEY, {}),
  change: (patch) => {
    set(patch);
    const { theme, fontSize, timeoutMs } = get();
    saveJson(STORAGE_KEY, { theme, fontSize, timeoutMs } satisfies Settings);
  },
}));
