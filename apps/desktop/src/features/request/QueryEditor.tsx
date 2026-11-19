import { Button } from "@roost/ui";

import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { setUrl } from "./editing";
import { type QueryRow, queryRows, withRows } from "./query";
import { VariableInput } from "../variables/VariableInput";

/** The URL's query parameters as a table; editing either one updates the other (FR-2.3). */
export function QueryEditor({ tab }: { tab: Tab }) {
  const { url } = tab.draft.request;
  const rows = queryRows(url);
  const change = (next: (rows: QueryRow[]) => QueryRow[]) => {
    useTabs.getState().edit(tab.id, setUrl(withRows(tab.draft.request.url, next)));
  };
  const update = (index: number, patch: Partial<QueryRow>) => {
    change((all) => all.map((r, i) => (i === index ? { ...r, ...patch } : r)));
  };
  return (
    <div className="flex flex-col gap-1">
      {rows.map((r, i) => (
        <div key={i} className="flex items-center gap-2">
          <input
            aria-label={`Query parameter ${String(i + 1)} name`}
            placeholder="Name"
            className="input w-48 font-mono"
            value={r.name}
            onChange={(e) => {
              update(i, { name: e.target.value });
            }}
          />
          <VariableInput
            aria-label={`Query parameter ${String(i + 1)} value`}
            placeholder="Value"
            className="input min-w-0 flex-1 font-mono"
            value={r.value}
            onValue={(value) => {
              update(i, { value });
            }}
          />
          <Button
            aria-label={`Remove query parameter ${String(i + 1)}`}
            onClick={() => {
              change((all) => all.filter((_, j) => j !== i));
            }}
          >
            ✕
          </Button>
        </div>
      ))}
      <div>
        <Button
          onClick={() => {
            change((all) => [...all, { name: "", value: "" }]);
          }}
        >
          Add parameter
        </Button>
      </div>
    </div>
  );
}
