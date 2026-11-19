import { Button, Modal, type ThemeSetting } from "@roost/ui";

import { useLayout } from "../layout/layoutStore";
import { FONT_SIZES, useSettings } from "./settingsStore";

const THEMES: [ThemeSetting, string][] = [
  ["system", "As the system"],
  ["dark", "Dark"],
  ["light", "Light"],
];

/** Theme, editor font size, where the response goes and the default timeout (FR-9.1). */
export function SettingsDialog({ onClose }: { onClose: () => void }) {
  const { theme, fontSize, timeoutMs, change } = useSettings();
  const split = useLayout((s) => s.split);
  const { placeResponse } = useLayout.getState();
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
          Response
          <select
            aria-label="Response placement"
            className="input"
            value={split}
            onChange={(e) => {
              placeResponse(e.target.value === "beside" ? "beside" : "below");
            }}
          >
            <option value="below">Below the request</option>
            <option value="beside">Beside the request</option>
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
