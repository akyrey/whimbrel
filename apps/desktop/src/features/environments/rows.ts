/** Name / value rows as edited, and the maps the files hold. Pure. */
export type Row = { key: string; value: string };

export const toRows = (map: Record<string, string>): Row[] =>
  Object.entries(map).map(([key, value]) => ({ key, value }));

/** Rows with an empty name are dropped; a later duplicate name wins. */
export const toMap = (rows: Row[]): Record<string, string> =>
  Object.fromEntries(rows.filter((r) => r.key.trim() !== "").map((r) => [r.key.trim(), r.value]));
