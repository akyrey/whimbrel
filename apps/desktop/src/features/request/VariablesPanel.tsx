import { Button } from "@roost/ui";

import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { type Variables, freshName, renamed, withValue, without } from "./variables";

/** The request's own `[variables]`: after the picked example, before the environment. */
export function VariablesPanel({ tab }: { tab: Tab }) {
  const { variables } = tab.draft;
  const entries = Object.entries(variables);
  const change = (next: (v: Variables) => Variables) => {
    useTabs.getState().edit(tab.id, (d) => ({ ...d, variables: next(d.variables) }));
  };
  return (
    <details className="text-sm" open={entries.length > 0}>
      <summary className="cursor-pointer text-zinc-400">
        Variables{entries.length > 0 ? ` (${String(entries.length)})` : ""}
      </summary>
      <div className="mt-2 flex flex-col gap-1">
        {entries.map(([name, value], i) => (
          <div key={i} className="flex items-center gap-2">
            <input
              aria-label={`Variable ${String(i + 1)} name`}
              className="input w-40 font-mono"
              value={name}
              onChange={(e) => {
                const to = e.target.value.trim();
                if (to !== "") change((v) => renamed(v, name, to));
              }}
            />
            <input
              aria-label={`Variable ${String(i + 1)} value`}
              className="input min-w-0 flex-1 font-mono"
              value={value}
              onChange={(e) => {
                change((v) => withValue(v, name, e.target.value));
              }}
            />
            <Button
              aria-label={`Remove variable ${name}`}
              onClick={() => {
                change((v) => without(v, name));
              }}
            >
              ✕
            </Button>
          </div>
        ))}
        <div>
          <Button
            onClick={() => {
              change((v) => withValue(v, freshName(v), ""));
            }}
          >
            Add variable
          </Button>
        </div>
      </div>
    </details>
  );
}
