/** Pretty-prints a JSON body. `{{variables}}` outside strings (`"id": {{id}}`) are not JSON,
 * so they stand in as placeholders while parsing and come back unchanged. */

const INDENT = 2;
const VARIABLE = /\{\{[^{}]*\}\}/y;

export type Beautified = { ok: true; text: string } | { ok: false; message: string };

/** Swaps each `{{…}}` outside a JSON string for a unique quoted token. */
function hideVariables(text: string): { text: string; hidden: Map<string, string> } {
  const hidden = new Map<string, string>();
  let out = "";
  let inString = false;
  let i = 0;
  while (i < text.length) {
    const c = text.charAt(i);
    if (inString) {
      out += c;
      if (c === "\\") {
        out += text.charAt(i + 1);
        i += 2;
        continue;
      }
      if (c === '"') inString = false;
      i += 1;
      continue;
    }
    if (c === '"') {
      inString = true;
      out += c;
      i += 1;
      continue;
    }
    VARIABLE.lastIndex = i;
    const found = VARIABLE.exec(text);
    if (found) {
      const token = `\u0000wbl${String(hidden.size)}\u0000`;
      hidden.set(JSON.stringify(token), found[0]);
      out += JSON.stringify(token);
      i += found[0].length;
      continue;
    }
    out += c;
    i += 1;
  }
  return { text: out, hidden };
}

export function beautifyJson(text: string): Beautified {
  if (text.trim() === "") return { ok: true, text };
  const { text: parsable, hidden } = hideVariables(text);
  let value: unknown;
  try {
    value = JSON.parse(parsable);
  } catch (caught) {
    return { ok: false, message: caught instanceof Error ? caught.message : "Not valid JSON" };
  }
  let pretty = JSON.stringify(value, null, INDENT);
  for (const [token, variable] of hidden) pretty = pretty.replace(token, variable);
  return { ok: true, text: pretty };
}
