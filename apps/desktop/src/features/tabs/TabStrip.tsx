import { ConfirmDialog } from "@roost/ui";
import { useState } from "react";

import { isDirty } from "./model";
import { useTabs } from "./tabsStore";
import { useSpecs } from "../openapi/specsStore";

/** Open requests; a dot marks unsaved edits; closing an unsaved tab asks first. */
export function TabStrip() {
  const tabs = useTabs((s) => s.tabs);
  const activeId = useTabs((s) => s.activeId);
  const [closing, setClosing] = useState<string | null>(null);
  const { activate, close, newScratch } = useTabs.getState();
  const askClose = (id: string) => {
    const tab = tabs.find((t) => t.id === id);
    if (tab && isDirty(tab)) setClosing(id);
    else close(id);
  };

  return (
    <div className="flex shrink-0 items-stretch border-b border-zinc-800 bg-zinc-900">
      <div role="tablist" aria-label="Open requests" className="flex min-w-0 overflow-x-auto">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            className={`flex items-center border-r border-zinc-800 text-sm ${tab.id === activeId ? "bg-zinc-950 text-zinc-100" : "text-zinc-400"}`}
          >
            <button
              type="button"
              role="tab"
              aria-selected={tab.id === activeId}
              className="max-w-48 truncate px-3 py-1.5"
              onClick={() => {
                // A tab click also leaves the spec browser, even for the tab already active.
                useSpecs.getState().close();
                activate(tab.id);
              }}
            >
              {tab.draft.name}
              {isDirty(tab) && (
                <span className="ml-1 text-amber-300" aria-label="unsaved">
                  •
                </span>
              )}
            </button>
            <button
              type="button"
              aria-label={`Close ${tab.draft.name}`}
              className="px-2 text-zinc-500 hover:text-zinc-200"
              onClick={() => {
                askClose(tab.id);
              }}
            >
              ✕
            </button>
          </div>
        ))}
      </div>
      <button
        type="button"
        aria-label="New request tab"
        className="px-3 text-zinc-400 hover:text-zinc-100"
        onClick={newScratch}
      >
        +
      </button>
      {closing && (
        <ConfirmDialog
          title="Close without saving?"
          message="The unsaved edits in this tab are lost."
          confirmLabel="Close"
          onCancel={() => {
            setClosing(null);
          }}
          onConfirm={() => {
            close(closing);
            setClosing(null);
          }}
        />
      )}
    </div>
  );
}
