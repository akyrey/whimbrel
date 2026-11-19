import { Button } from "@roost/ui";

import type { StoredRequest } from "../../ipc";
import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { addHeader, removeHeader, updateHeader } from "./editing";
import { VariableInput } from "../variables/VariableInput";

/** The request's headers: name, value, on/off; order is kept, duplicates allowed. */
export function HeadersEditor({ tab }: { tab: Tab }) {
  const { headers } = tab.draft.request;
  const edit = (change: (draft: StoredRequest) => StoredRequest) => {
    useTabs.getState().edit(tab.id, change);
  };

  return (
    <div className="flex flex-col gap-1 text-sm">
      {headers.map((header, index) => (
        <div key={index} className="flex items-center gap-2">
          <input
            type="checkbox"
            aria-label={`Send header ${String(index + 1)}`}
            checked={header.enabled}
            onChange={(e) => {
              edit(updateHeader(index, { enabled: e.target.checked }));
            }}
          />
          <input
            aria-label={`Header ${String(index + 1)} name`}
            placeholder="Name"
            className="input w-56 font-mono"
            value={header.name}
            onChange={(e) => {
              edit(updateHeader(index, { name: e.target.value }));
            }}
          />
          <VariableInput
            aria-label={`Header ${String(index + 1)} value`}
            placeholder="Value"
            className="input min-w-0 flex-1 font-mono"
            value={header.value}
            onValue={(value) => {
              edit(updateHeader(index, { value }));
            }}
          />
          <Button
            aria-label={`Remove header ${String(index + 1)}`}
            onClick={() => {
              edit(removeHeader(index));
            }}
          >
            ✕
          </Button>
        </div>
      ))}
      <div>
        <Button
          onClick={() => {
            edit(addHeader);
          }}
        >
          Add header
        </Button>
      </div>
    </div>
  );
}
