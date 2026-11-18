import { Button } from "@roost/ui";

import { useRequest } from "./requestStore";

/** The request's headers: name, value, on/off; order is kept, duplicates allowed. */
export function HeadersEditor() {
  const headers = useRequest((s) => s.request.headers);
  const { addHeader, updateHeader, removeHeader } = useRequest.getState();

  return (
    <div className="flex flex-col gap-1 text-sm">
      {headers.map((header, index) => (
        <div key={index} className="flex items-center gap-2">
          <input
            type="checkbox"
            aria-label={`Send header ${String(index + 1)}`}
            checked={header.enabled}
            onChange={(e) => {
              updateHeader(index, { enabled: e.target.checked });
            }}
          />
          <input
            aria-label={`Header ${String(index + 1)} name`}
            placeholder="Name"
            className="input w-56 font-mono"
            value={header.name}
            onChange={(e) => {
              updateHeader(index, { name: e.target.value });
            }}
          />
          <input
            aria-label={`Header ${String(index + 1)} value`}
            placeholder="Value"
            className="input min-w-0 flex-1 font-mono"
            value={header.value}
            onChange={(e) => {
              updateHeader(index, { value: e.target.value });
            }}
          />
          <Button
            aria-label={`Remove header ${String(index + 1)}`}
            onClick={() => {
              removeHeader(index);
            }}
          >
            ✕
          </Button>
        </div>
      ))}
      <div>
        <Button onClick={addHeader}>Add header</Button>
      </div>
    </div>
  );
}
