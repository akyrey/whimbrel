import { Button, Skeleton, toast } from "@roost/ui";
import { Suspense, lazy } from "react";

import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { type BodyChoice, choiceOf, setBodyKind, setBodyText } from "./editing";
import { beautifyJson } from "./beautify";
import { FieldsEditor, FileBodyEditor } from "./FormEditor";

/** CodeMirror loads when a body is first edited, not at start. */
const CodeEditor = lazy(async () => ({ default: (await import("@roost/editor")).CodeEditor }));

const KINDS: { value: BodyChoice; label: string }[] = [
  { value: "none", label: "No body" },
  { value: "json", label: "JSON" },
  { value: "xml", label: "XML" },
  { value: "text", label: "Text" },
  { value: "form", label: "Form (URL-encoded)" },
  { value: "multipart", label: "Multipart form" },
  { value: "file", label: "File" },
];

export function BodyEditor({ tab }: { tab: Tab }) {
  const { body } = tab.draft.request;
  const { edit } = useTabs.getState();
  const kind = choiceOf(body);
  const change = (c: Parameters<typeof edit>[1]) => {
    edit(tab.id, c);
  };

  const beautify = (text: string) => {
    const result = beautifyJson(text);
    if (!result.ok) {
      toast.error(`Not valid JSON: ${result.message}`);
      return;
    }
    if (result.text === text) return;
    // The editor follows the draft's text, so this is one edit that undo takes back.
    edit(tab.id, setBodyText(result.text));
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <div className="flex items-center gap-2">
        <select
          aria-label="Body type"
          className="input w-48"
          value={kind}
          onChange={(e) => {
            edit(tab.id, setBodyKind(e.target.value as BodyChoice));
          }}
        >
          {KINDS.map((k) => (
            <option key={k.value} value={k.value}>
              {k.label}
            </option>
          ))}
        </select>
        {body.type === "text" && body.kind === "json" && (
          <Button
            title="Pretty-print the JSON"
            onClick={() => {
              beautify(body.text);
            }}
          >
            Beautify
          </Button>
        )}
      </div>
      {body.type === "text" && (
        <Suspense fallback={<Skeleton rows={4} label="Loading the editor" />}>
          <CodeEditor
            // A new kind is a new language: rebuild the editor, keeping the text.
            key={`${tab.id}:${body.kind}`}
            initialText={body.text}
            text={body.text}
            label="Request body"
            language={body.kind}
            wrap
            invalid={false}
            onChange={(text) => {
              edit(tab.id, setBodyText(text));
            }}
          />
        </Suspense>
      )}
      {(body.type === "form" || body.type === "multipart") && (
        <FieldsEditor fields={body.fields} multipart={body.type === "multipart"} edit={change} />
      )}
      {body.type === "file" && <FileBodyEditor path={body.path} edit={change} />}
    </div>
  );
}
