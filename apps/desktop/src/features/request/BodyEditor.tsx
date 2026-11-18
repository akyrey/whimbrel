import { Skeleton } from "@roost/ui";
import { Suspense, lazy } from "react";

import type { BodyKind } from "../../ipc";
import { useRequest } from "./requestStore";

/** CodeMirror loads when a body is first edited, not at start. */
const CodeEditor = lazy(async () => ({ default: (await import("@roost/editor")).CodeEditor }));

const KINDS: { value: BodyKind | "none"; label: string }[] = [
  { value: "none", label: "No body" },
  { value: "json", label: "JSON" },
  { value: "xml", label: "XML" },
  { value: "text", label: "Text" },
];

export function BodyEditor() {
  const body = useRequest((s) => s.request.body);
  const { setBodyKind, setBodyText } = useRequest.getState();
  const kind = body.type === "text" ? body.kind : "none";

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <select
        aria-label="Body type"
        className="input w-36"
        value={kind}
        onChange={(e) => {
          setBodyKind(e.target.value as BodyKind | "none");
        }}
      >
        {KINDS.map((k) => (
          <option key={k.value} value={k.value}>
            {k.label}
          </option>
        ))}
      </select>
      {body.type === "text" && (
        <Suspense fallback={<Skeleton rows={4} label="Loading the editor" />}>
          <CodeEditor
            // A new kind is a new language: rebuild the editor, keeping the text.
            key={body.kind}
            initialText={body.text}
            label="Request body"
            language={body.kind}
            wrap
            invalid={false}
            onChange={setBodyText}
          />
        </Suspense>
      )}
    </div>
  );
}
