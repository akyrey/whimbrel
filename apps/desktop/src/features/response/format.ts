/** How a response body is shown: pretty-printed when its type says JSON or XML. Pure. */
import type { ResponseBody } from "../../ipc";

export type ShownBody =
  | { kind: "empty" }
  | { kind: "binary"; size: number }
  | { kind: "code"; language: "json" | "xml" | "text"; text: string };

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
  if (body.text === null) return { kind: "binary", size: body.size };
  if (body.text === "") return { kind: "empty" };
  const type = body.contentType ?? "";
  if (JSON_TYPE.test(type)) {
    const pretty = prettyJson(body.text);
    // A body that says JSON but does not parse is shown as it came.
    return pretty === null
      ? { kind: "code", language: "text", text: body.text }
      : { kind: "code", language: "json", text: pretty };
  }
  if (XML_TYPE.test(type)) return { kind: "code", language: "xml", text: body.text };
  return { kind: "code", language: "text", text: body.text };
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
