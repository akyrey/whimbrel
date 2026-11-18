/** How a response body is shown (FR-3.2, FR-3.3). Pure. */
import type { ResponseBody, ResponseHeader } from "../../ipc";

/** Bodies longer than this are shown as they came: pretty-printing them would block the window. */
export const PRETTY_LIMIT = 2 * 1024 * 1024;
/** Bytes shown in the hex view. */
export const HEX_LIMIT = 64 * 1024;

export type ShownBody =
  | { kind: "empty" }
  | { kind: "binary"; size: number; base64: string }
  | { kind: "code"; language: "json" | "xml" | "text"; text: string; pretty: boolean };

const JSON_TYPE = /[/+]json\b/i;
const XML_TYPE = /[/+]xml\b/i;
const INDENT = 2;

function prettyJson(text: string): string | null {
  try {
    return JSON.stringify(JSON.parse(text), null, INDENT);
  } catch {
    return null;
  }
}

export function showBody(body: ResponseBody): ShownBody {
  if (body.text === null) return { kind: "binary", size: body.size, base64: body.base64 ?? "" };
  if (body.text === "") return { kind: "empty" };
  const type = body.contentType ?? "";
  if (JSON_TYPE.test(type)) {
    if (body.truncated || body.text.length > PRETTY_LIMIT) {
      return { kind: "code", language: "json", text: body.text, pretty: false };
    }
    const pretty = prettyJson(body.text);
    // A body that says JSON but does not parse is shown as it came.
    return pretty === null
      ? { kind: "code", language: "text", text: body.text, pretty: false }
      : { kind: "code", language: "json", text: pretty, pretty: true };
  }
  const language = XML_TYPE.test(type) ? "xml" : "text";
  return { kind: "code", language, text: body.text, pretty: false };
}

function bytesOf(base64: string): Uint8Array {
  const raw = atob(base64);
  return Uint8Array.from(raw, (c) => c.charCodeAt(0));
}

const HEX_ROW = 16;
const HEX_WIDTH = HEX_ROW * 3;
const PRINTABLE_FIRST = 0x20;
const PRINTABLE_LAST = 0x7e;

/** `00000000  48 65 6c …  |Hel…|`, at most `HEX_LIMIT` bytes. */
export function hexDump(base64: string): string {
  const bytes = bytesOf(base64).slice(0, HEX_LIMIT);
  const lines: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += HEX_ROW) {
    const row = Array.from(bytes.slice(offset, offset + HEX_ROW));
    const hex = row.map((b) => b.toString(16).padStart(2, "0"));
    const left = hex.slice(0, HEX_ROW / 2).join(" ");
    const right = hex.slice(HEX_ROW / 2).join(" ");
    const text = row
      .map((b) => (b >= PRINTABLE_FIRST && b <= PRINTABLE_LAST ? String.fromCharCode(b) : "."))
      .join("");
    const columns = right ? `${left}  ${right}` : left;
    lines.push(`${offset.toString(16).padStart(8, "0")}  ${columns.padEnd(HEX_WIDTH)}  |${text}|`);
  }
  return lines.join("\n");
}

function base64OfText(text: string): string {
  const bytes = new TextEncoder().encode(text);
  return btoa(Array.from(bytes, (b) => String.fromCharCode(b)).join(""));
}

/** A data URL for an image body shown whole; `null` for anything else. */
export function imageSource(body: ResponseBody): string | null {
  const type = body.contentType?.split(";")[0]?.trim().toLowerCase() ?? "";
  if (!type.startsWith("image/") || body.truncated) return null;
  const base64 = body.base64 ?? (body.text === null ? null : base64OfText(body.text));
  return base64 === null ? null : `data:${type};base64,${base64}`;
}

export type JsonTable = { columns: string[]; rows: unknown[][] };

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** A JSON array of objects as a table (columns: every key, in first-seen order). */
export function jsonTable(text: string): JsonTable | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (!Array.isArray(parsed) || parsed.length === 0 || !parsed.every(isRecord)) return null;
  const records = parsed;
  const columns = [...new Set(records.flatMap((r) => Object.keys(r)))];
  return { columns, rows: records.map((r) => columns.map((c) => r[c])) };
}

export type Cookie = { name: string; value: string; attributes: string };

/** The cookies the response sets. */
export function cookiesOf(headers: ResponseHeader[]): Cookie[] {
  return headers
    .filter((h) => h.name.toLowerCase() === "set-cookie")
    .map((h) => {
      const [pair = "", ...attributes] = h.value.split(";").map((p) => p.trim());
      const at = pair.indexOf("=");
      return {
        name: at < 0 ? pair : pair.slice(0, at),
        value: at < 0 ? "" : pair.slice(at + 1),
        attributes: attributes.join("; "),
      };
    });
}

const UNITS = ["B", "KB", "MB", "GB"];
const STEP = 1024;

/** `512 B`, `1.5 KB`, `10.0 MB`. */
export function formatSize(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= STEP && unit < UNITS.length - 1) {
    value /= STEP;
    unit += 1;
  }
  return unit === 0 ? `${String(value)} B` : `${value.toFixed(1)} ${UNITS[unit] ?? ""}`;
}

/** The colour of a status: success, redirect, client error, server error. */
export function statusTone(status: number): "ok" | "redirect" | "client" | "server" {
  if (status >= 500) return "server";
  if (status >= 400) return "client";
  if (status >= 300) return "redirect";
  return "ok";
}
