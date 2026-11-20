import { Button } from "@roost/ui";

import { openCollection } from "../collections/actions";
import { shortcutLabel } from "../shortcuts/registry";
import { useTabs } from "../tabs/tabsStore";
import { type AppDialog, useDialogs } from "./dialogsStore";

const IMPORTS: [AppDialog, string][] = [
  ["openapi", "Import an OpenAPI document…"],
  ["interop", "Import from Postman or .http…"],
  ["curl", "Import a cURL command…"],
];

/** What to do with no tab open (task 9.7). */
export function EmptyState() {
  return (
    <section
      aria-label="Get started"
      className="flex flex-1 flex-col items-center justify-center gap-4 text-sm text-zinc-400"
    >
      <p>Open a request from a collection, or start one here.</p>
      <div className="flex flex-col items-stretch gap-2">
        <Button variant="primary" onClick={useTabs.getState().newScratch}>
          New request
          <kbd aria-hidden="true" className="ml-3 font-sans text-xs opacity-70">
            {shortcutLabel("new-tab")}
          </kbd>
        </Button>
        <Button onClick={() => void openCollection()}>Open a collection…</Button>
        {IMPORTS.map(([dialog, label]) => (
          <Button
            key={dialog}
            onClick={() => {
              useDialogs.getState().show(dialog);
            }}
          >
            {label}
          </Button>
        ))}
      </div>
    </section>
  );
}
