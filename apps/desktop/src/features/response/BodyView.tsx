import { Button, Skeleton, copyWithToast, toast } from "@roost/ui";
import { Suspense, lazy, useMemo, useState } from "react";

import { type Response, chooseSaveFile, responseSave, toAppError } from "../../ipc";
import { PRETTY_LIMIT, formatSize, hexDump, imageSource, jsonTable, showBody } from "./format";
import { JsonGrid } from "./JsonGrid";

const ReadOnlyCode = lazy(async () => ({ default: (await import("@roost/editor")).ReadOnlyCode }));

type View = "pretty" | "raw" | "table" | "hex" | "image";

const LABELS: Record<View, string> = {
  pretty: "Pretty",
  raw: "Raw",
  table: "Table",
  hex: "Hex",
  image: "Image",
};

async function save(response: Response) {
  const { body } = response;
  const name = response.url.split("?")[0]?.split("/").pop() || "response";
  try {
    const destination = await chooseSaveFile("Save the response body", name);
    if (!destination) return;
    await responseSave({
      destination,
      ...(body.file !== null
        ? { file: body.file }
        : body.base64 !== null
          ? { base64: body.base64 }
          : { text: body.text ?? "" }),
    });
    toast.success(`Saved to ${destination}`);
  } catch (caught) {
    toast.error(`Could not save the body: ${toAppError(caught).message}`);
  }
}

function views(response: Response, table: boolean, image: boolean): View[] {
  const shown = showBody(response.body);
  const out: View[] = [];
  if (shown.kind === "code")
    out.push(...(shown.pretty ? (["pretty", "raw"] as const) : (["raw"] as const)));
  if (table) out.push("table");
  if (image) out.push("image");
  if (shown.kind === "binary") out.push("hex");
  return out;
}

/** Wrapping makes the editor lay out a whole line at once: a long single-line body would block
 * the window, so large bodies never wrap. */
const canWrap = (text: string) => text.length <= PRETTY_LIMIT;

function Code({ text, language, wrap }: { text: string; language: string; wrap: boolean }) {
  return (
    <Suspense fallback={<Skeleton rows={6} label="Loading the body" />}>
      <ReadOnlyCode
        text={text}
        label="Response body"
        language={language}
        wrap={wrap && canWrap(text)}
      />
    </Suspense>
  );
}

/** The body, in each view it allows; a notice and "Save to file" when only its start is shown. */
export function BodyView({ response }: { response: Response }) {
  const shown = useMemo(() => showBody(response.body), [response.body]);
  const table = useMemo(
    () =>
      shown.kind === "code" && shown.language === "json" && !response.body.truncated
        ? jsonTable(response.body.text ?? "")
        : null,
    [shown, response.body],
  );
  const image = imageSource(response.body);
  const available = views(response, table !== null, image !== null);
  const [view, setView] = useState<View>(available[0] ?? "raw");
  const current = available.includes(view) ? view : (available[0] ?? "raw");
  const [wrap, setWrap] = useState(true);
  const codeText =
    shown.kind !== "code"
      ? null
      : current === "pretty"
        ? shown.text
        : current === "raw"
          ? (response.body.text ?? "")
          : null;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-xs">
        {available.length > 1 && (
          <div role="radiogroup" aria-label="Body view" className="flex gap-1">
            {available.map((v) => (
              <button
                key={v}
                type="button"
                role="radio"
                aria-checked={current === v}
                className={`rounded px-2 py-0.5 ${current === v ? "bg-zinc-700 text-zinc-100" : "text-zinc-400"}`}
                onClick={() => {
                  setView(v);
                }}
              >
                {LABELS[v]}
              </button>
            ))}
          </div>
        )}
        {codeText !== null && (
          <label className="ml-auto flex items-center gap-1 text-zinc-400">
            <input
              type="checkbox"
              checked={wrap && canWrap(codeText)}
              disabled={!canWrap(codeText)}
              title={canWrap(codeText) ? undefined : "Too large to wrap"}
              onChange={(e) => {
                setWrap(e.target.checked);
              }}
            />
            Wrap
          </label>
        )}
        {codeText !== null && (
          <Button onClick={() => void copyWithToast(codeText, "the body")}>Copy body</Button>
        )}
        {shown.kind !== "empty" && (
          <Button
            className={codeText === null ? "ml-auto" : ""}
            onClick={() => void save(response)}
          >
            Save to file
          </Button>
        )}
      </div>
      {response.body.truncated && (
        <p role="note" className="text-xs text-amber-300">
          Showing the first part of {formatSize(response.body.size)}.
          {response.body.file !== null ? " Save to file for all of it." : " The rest was not read."}
        </p>
      )}
      {shown.kind === "empty" && <p className="text-sm text-zinc-500">No body.</p>}
      {shown.kind === "code" && current === "pretty" && (
        <Code text={shown.text} language={shown.language} wrap={wrap} />
      )}
      {shown.kind === "code" && current === "raw" && (
        <Code
          text={response.body.text ?? ""}
          language={shown.language === "json" && !shown.pretty ? "json" : "text"}
          wrap={wrap}
        />
      )}
      {current === "table" && table && <JsonGrid table={table} />}
      {current === "image" && image && (
        <img src={image} alt="The response body" className="max-h-96 max-w-full object-contain" />
      )}
      {current === "hex" && shown.kind === "binary" && (
        <pre aria-label="Body as hex" className="overflow-auto font-mono text-xs text-zinc-300">
          {hexDump(shown.base64)}
        </pre>
      )}
    </div>
  );
}
