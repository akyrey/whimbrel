import type { RequestSettings } from "../../ipc";
import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";

/** The request's own timeout, redirects and certificate check (FR-2.1). */
export function SettingsPanel({ tab }: { tab: Tab }) {
  const { settings } = tab.draft;
  const change = (patch: Partial<RequestSettings>) => {
    useTabs.getState().edit(tab.id, (d) => ({ ...d, settings: { ...d.settings, ...patch } }));
  };
  return (
    <div className="flex flex-wrap items-center gap-4">
      <label className="flex items-center gap-2">
        Timeout (ms)
        <input
          aria-label="Timeout in milliseconds"
          type="number"
          min={1}
          placeholder="30000"
          className="input w-28"
          value={settings.timeoutMs ?? ""}
          onChange={(e) => {
            const ms = Number.parseInt(e.target.value, 10);
            change({ timeoutMs: Number.isFinite(ms) && ms > 0 ? ms : null });
          }}
        />
      </label>
      <label className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={settings.followRedirects}
          onChange={(e) => {
            change({ followRedirects: e.target.checked });
          }}
        />
        Follow redirects
      </label>
      <label className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={settings.verifyTls}
          onChange={(e) => {
            change({ verifyTls: e.target.checked });
          }}
        />
        Verify certificates
      </label>
      {!settings.verifyTls && (
        <span className="text-xs text-amber-300">
          Any certificate is accepted: only for servers you trust.
        </span>
      )}
    </div>
  );
}
