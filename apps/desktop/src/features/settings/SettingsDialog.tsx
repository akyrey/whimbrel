import { Button, Modal, type ThemeSetting } from "@roost/ui";

import { FONT_SIZES, useSettings } from "./settingsStore";

const THEMES: [ThemeSetting, string][] = [
  ["system", "As the system"],
  ["dark", "Dark"],
  ["light", "Light"],
];

/** Theme, editor font size and the default timeout (FR-9.1). */
export function SettingsDialog({ onClose }: { onClose: () => void }) {
  const { theme, fontSize, timeoutMs, change } = useSettings();
  return (
    <Modal title="Settings" onClose={onClose} footer={<Button onClick={onClose}>Done</Button>}>
      <div className="flex flex-col gap-3 text-sm">
        <label className="flex items-center justify-between gap-4">
          Theme
          <select
            aria-label="Theme"
            className="input"
            value={theme}
            onChange={(e) => {
              change({ theme: e.target.value as ThemeSetting });
            }}
          >
            {THEMES.map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center justify-between gap-4">
          Editor font size
          <select
            aria-label="Editor font size"
            className="input"
            value={fontSize}
            onChange={(e) => {
              change({ fontSize: Number(e.target.value) });
            }}
          >
            {FONT_SIZES.map((size) => (
              <option key={size} value={size}>
                {size} px
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center justify-between gap-4">
          Default timeout (ms)
          <input
            aria-label="Default timeout in milliseconds"
            type="number"
            min={1}
            placeholder="30000"
            className="input w-32"
            value={timeoutMs ?? ""}
            onChange={(e) => {
              const ms = Number.parseInt(e.target.value, 10);
              change({ timeoutMs: Number.isFinite(ms) && ms > 0 ? ms : null });
            }}
          />
        </label>
        <p className="text-xs text-zinc-500">A request's own timeout wins over the default.</p>
      </div>
    </Modal>
  );
}
