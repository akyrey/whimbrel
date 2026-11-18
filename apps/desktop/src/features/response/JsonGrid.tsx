import { type CellAdapter, DataGrid, type GridColumnBase } from "@roost/grid";

import type { JsonTable } from "./format";

type Column = GridColumnBase;

/** JSON values in @roost/grid: nested values as compact JSON, missing ones empty. */
const cells: CellAdapter<unknown, Column> = {
  present: (value) => {
    const base = { cut: false, alignRight: false };
    if (value === undefined) return { ...base, kind: "empty", text: "" };
    if (value === null) return { ...base, kind: "null", text: "null" };
    if (typeof value === "boolean")
      return { ...base, kind: value ? "true" : "false", text: String(value) };
    if (typeof value === "number")
      return { ...base, kind: "number", text: String(value), alignRight: true };
    if (typeof value === "string") return { ...base, kind: "text", text: value };
    return { ...base, kind: "json", text: JSON.stringify(value) };
  },
  widthText: (value) => (typeof value === "string" ? value : JSON.stringify(value)),
  copyText: (value) =>
    value === undefined ? null : typeof value === "string" ? value : JSON.stringify(value),
  header: () => ({ detail: "", badges: [] }),
};

/** A JSON array of objects as a table (FR-3.3). */
export function JsonGrid({ table }: { table: JsonTable }) {
  const columns = table.columns.map((name) => ({ name }));
  return (
    <div className="h-80" aria-label="Body as a table">
      <DataGrid columns={columns} rows={table.rows} cells={cells} />
    </div>
  );
}
