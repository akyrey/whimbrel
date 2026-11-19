import { Button, Skeleton } from "@roost/ui";
import { Suspense, lazy, useState } from "react";

import type { MediaType, Operation, Parameter } from "../../ipc";
import { useTabs } from "../tabs/tabsStore";
import { SchemaTree } from "./SchemaTree";
import { typeLabel } from "./browse";
import { useSpecs } from "./specsStore";

const ReadOnlyCode = lazy(async () => ({ default: (await import("@roost/editor")).ReadOnlyCode }));

const pretty = (value: unknown): string =>
  typeof value === "string" ? value : JSON.stringify(value, null, 2);

function ParameterTable({ parameters }: { parameters: Parameter[] }) {
  if (parameters.length === 0) return null;
  return (
    <section aria-label="Parameters">
      <h3 className="mb-1 font-medium">Parameters</h3>
      <table className="w-full text-left text-sm">
        <thead className="text-xs text-zinc-500">
          <tr>
            <th>Name</th>
            <th>In</th>
            <th>Type</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          {parameters.map((p) => (
            <tr key={`${p.location}:${p.name}`} className="border-t border-zinc-800 align-top">
              <td className="font-mono">
                {p.name}
                {p.required && <span className="ml-1 text-xs text-red-400">required</span>}
                {p.deprecated && <span className="ml-1 text-xs text-amber-300">deprecated</span>}
              </td>
              <td>{p.location}</td>
              <td className="text-sky-300">{p.schema == null ? "" : typeLabel(p.schema)}</td>
              <td className="text-zinc-400">{p.description}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

function examplesOf(media: MediaType): { name: string; text: string }[] {
  const named = media.examples
    .filter((e) => e.serialized != null || e.value != null)
    .map((e) => ({ name: e.summary ?? e.name, text: e.serialized ?? pretty(e.value) }));
  return media.example == null
    ? named
    : [{ name: "example", text: pretty(media.example) }, ...named];
}

/** Media types as tabs, each with its schema tree and examples. */
function Content({ content, label }: { content: MediaType[]; label: string }) {
  const [index, setIndex] = useState(0);
  const media = content[Math.min(index, content.length - 1)];
  if (!media) return null;
  const examples = examplesOf(media);
  return (
    <div className="flex flex-col gap-2">
      <div role="tablist" aria-label={`${label} media types`} className="flex gap-2 text-xs">
        {content.map((m, i) => (
          <button
            key={m.mediaType}
            type="button"
            role="tab"
            aria-selected={i === index}
            className={i === index ? "text-zinc-100 underline" : "text-zinc-400"}
            onClick={() => {
              setIndex(i);
            }}
          >
            {m.mediaType}
          </button>
        ))}
      </div>
      {media.schema != null && <SchemaTree schema={media.schema} label={label} />}
      {media.itemSchema != null && <SchemaTree schema={media.itemSchema} label="each item" />}
      {examples.map((e) => (
        <div key={e.name} className="flex flex-col gap-1">
          <span className="text-xs text-zinc-500">{e.name}</span>
          <Suspense fallback={<Skeleton rows={3} />}>
            <ReadOnlyCode
              text={e.text}
              label={`Example ${e.name}`}
              language={media.mediaType.includes("json") ? "json" : "text"}
              wrap
            />
          </Suspense>
        </div>
      ))}
    </div>
  );
}

type Props = { operation: Operation; root: string; file: string | null; readOnly?: boolean };

/** One operation: what it takes and what it answers; "Open request" when one was generated. */
export function OperationPage({ operation, root, file, readOnly = false }: Props) {
  return (
    <article aria-label={`Operation ${operation.id}`} className="flex flex-col gap-4">
      <header className="flex flex-wrap items-center gap-3">
        <span className="rounded bg-zinc-800 px-2 py-0.5 font-mono text-xs">
          {operation.method}
        </span>
        <h2 className="font-mono text-base">{operation.path}</h2>
        {operation.deprecated && <span className="text-xs text-amber-300">deprecated</span>}
        {readOnly && <span className="text-xs text-zinc-500">webhook (sent by the API)</span>}
        {file !== null && (
          <Button
            className="ml-auto"
            onClick={() => {
              useSpecs.getState().close();
              void useTabs.getState().openFile({ root, path: file });
            }}
          >
            Open request
          </Button>
        )}
      </header>
      {operation.summary && <p className="font-medium">{operation.summary}</p>}
      {operation.description && (
        <p className="whitespace-pre-wrap text-sm text-zinc-400">{operation.description}</p>
      )}
      <ParameterTable parameters={operation.parameters} />
      {operation.requestBody && operation.requestBody.content.length > 0 && (
        <section aria-label="Request body">
          <h3 className="mb-1 font-medium">
            Request body{operation.requestBody.required ? "" : " (optional)"}
          </h3>
          <Content content={operation.requestBody.content} label="body" />
        </section>
      )}
      <section aria-label="Responses" className="flex flex-col gap-3">
        <h3 className="font-medium">Responses</h3>
        {operation.responses.map((r) => (
          <details key={r.status} open={r.status.startsWith("2")}>
            <summary className="cursor-pointer">
              <span className="font-mono">{r.status}</span>{" "}
              <span className="text-zinc-400">{r.summary}</span>
            </summary>
            <div className="ml-4 mt-2">
              <Content content={r.content} label={`response ${r.status}`} />
            </div>
          </details>
        ))}
      </section>
    </article>
  );
}
