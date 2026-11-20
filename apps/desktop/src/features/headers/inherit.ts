/**
 * Headers a request inherits from its folders and collection (D-H22), as
 * crates/wbl-core/src/inherit.rs applies them: the collection's first, then each folder's
 * inwards; a deeper header (enabled or not) replaces every inherited one of the same name,
 * ignoring case. Pure; the backend is what sends them.
 */
import type { Header } from "../../ipc";
import type { InheritSource } from "../auth/inheritance";

/** One level's headers, innermost first like `inheritedFrom`. */
export type HeadersLevel = { source: InheritSource; headers: Header[] };

export type InheritedRow = { header: Header; source: InheritSource; overridden: boolean };

const sameName = (a: Header) => (b: Header) => a.name.toLowerCase() === b.name.toLowerCase();

const layer = (inherited: Header[], level: Header[]): Header[] => [
  ...inherited.filter((h) => !level.some(sameName(h))),
  ...level,
];

/** The headers sent: `levels` (innermost first) laid under `own`. */
export function withInherited(levels: Header[][], own: Header[]): Header[] {
  const inherited = [...levels].reverse().reduce(layer, []);
  return layer(inherited, own);
}

/** Every inherited header, outermost first, marked when a deeper level or `own` replaces it. */
export function inheritedRows(levels: HeadersLevel[], own: Header[]): InheritedRow[] {
  const outward = [...levels].reverse();
  return outward.flatMap((level, depth) => {
    const deeper = [...outward.slice(depth + 1).flatMap((l) => l.headers), ...own];
    return level.headers.map((header) => ({
      header,
      source: level.source,
      overridden: deeper.some(sameName(header)),
    }));
  });
}

const EMPTY_HEADER: Header = { name: "", value: "", enabled: true };

export const addHeader = (headers: Header[]): Header[] => [...headers, EMPTY_HEADER];

export const updateHeader =
  (index: number, change: Partial<Header>) =>
  (headers: Header[]): Header[] =>
    headers.map((h, i) => (i === index ? { ...h, ...change } : h));

export const removeHeader =
  (index: number) =>
  (headers: Header[]): Header[] =>
    headers.filter((_, i) => i !== index);
