import { Button } from "@roost/ui";

import type { Row } from "./rows";

type Props = {
  label: string;
  rows: Row[];
  onChange: (rows: Row[]) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
};

/** Editable name / value pairs (variables, overrides). */
export function KeyValueTable({
  label,
  rows,
  onChange,
  keyPlaceholder = "Name",
  valuePlaceholder = "Value",
}: Props) {
  const update = (index: number, change: Partial<Row>) => {
    onChange(rows.map((r, i) => (i === index ? { ...r, ...change } : r)));
  };
  return (
    <fieldset className="flex flex-col gap-1 text-sm">
      <legend className="mb-1 text-xs font-semibold uppercase tracking-wide text-zinc-400">
        {label}
      </legend>
      {rows.map((row, index) => (
        <div key={index} className="flex gap-2">
          <input
            aria-label={`${label} ${String(index + 1)} name`}
            placeholder={keyPlaceholder}
            className="input w-44 font-mono"
            value={row.key}
            onChange={(e) => {
              update(index, { key: e.target.value });
            }}
          />
          <input
            aria-label={`${label} ${String(index + 1)} value`}
            placeholder={valuePlaceholder}
            className="input min-w-0 flex-1 font-mono"
            value={row.value}
            onChange={(e) => {
              update(index, { value: e.target.value });
            }}
          />
          <Button
            aria-label={`Remove ${label} ${String(index + 1)}`}
            onClick={() => {
              onChange(rows.filter((_, i) => i !== index));
            }}
          >
            ✕
          </Button>
        </div>
      ))}
      <div>
        <Button
          onClick={() => {
            onChange([...rows, { key: "", value: "" }]);
          }}
        >
          Add
        </Button>
      </div>
    </fieldset>
  );
}
