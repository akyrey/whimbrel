/** The query string of a URL as rows, and back (FR-2.3). Text is kept as written: `{{vars}}`
 * and percent-escapes stay, so the URL and the table always say the same thing. Pure. */

/** `bare`: written without `=` (`?flag`), kept so the URL is rewritten as it was typed. */
export type QueryRow = { name: string; value: string; bare?: boolean };

type Split = { base: string; rows: QueryRow[]; hash: string };

export function splitUrl(url: string): Split {
  const hashAt = url.indexOf("#");
  const hash = hashAt < 0 ? "" : url.slice(hashAt);
  const beforeHash = hashAt < 0 ? url : url.slice(0, hashAt);
  const queryAt = beforeHash.indexOf("?");
  if (queryAt < 0) return { base: beforeHash, rows: [], hash };
  const query = beforeHash.slice(queryAt + 1);
  const rows = query === "" ? [] : query.split("&").map(row);
  return { base: beforeHash.slice(0, queryAt), rows, hash };
}

function row(part: string): QueryRow {
  const eq = part.indexOf("=");
  return eq < 0
    ? { name: part, value: "", bare: true }
    : { name: part.slice(0, eq), value: part.slice(eq + 1) };
}

export function joinUrl({ base, rows, hash }: Split): string {
  const query = rows
    .map((r) => (r.bare === true && r.value === "" ? r.name : `${r.name}=${r.value}`))
    .join("&");
  return `${base}${rows.length > 0 ? `?${query}` : ""}${hash}`;
}

export const queryRows = (url: string): QueryRow[] => splitUrl(url).rows;

export function withRows(url: string, change: (rows: QueryRow[]) => QueryRow[]): string {
  const split = splitUrl(url);
  return joinUrl({ ...split, rows: change(split.rows) });
}
