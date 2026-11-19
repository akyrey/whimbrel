import { useMemo, useState } from "react";

import type { Api, Operation } from "../../ipc";
import { type TagNode, tagTree } from "./browse";
import { type SpecFocus, useSpecs } from "./specsStore";

function OperationLink({
  op,
  focus,
  kind,
}: {
  op: Operation;
  focus: SpecFocus;
  kind: "operation" | "webhook";
}) {
  const current = focus.kind === kind && focus.id === op.id;
  return (
    <li>
      <button
        type="button"
        aria-current={current ? "page" : undefined}
        className={`flex w-full gap-2 truncate px-1 text-left ${current ? "bg-zinc-800 text-zinc-100" : "text-zinc-300 hover:bg-zinc-800/60"} ${op.deprecated ? "line-through" : ""}`}
        onClick={() => {
          useSpecs.getState().focus({ kind, id: op.id });
        }}
      >
        <span className="w-14 shrink-0 font-mono text-xs text-zinc-500">{op.method}</span>
        <span className="truncate">{op.summary ?? op.path}</span>
      </button>
    </li>
  );
}

function TagBranch({
  node,
  focus,
  filtering,
}: {
  node: TagNode;
  focus: SpecFocus;
  filtering: boolean;
}) {
  return (
    <li>
      <details open={filtering}>
        <summary className="cursor-pointer text-zinc-200">{node.label}</summary>
        <ul className="ml-2">
          {node.children.map((c) => (
            <TagBranch key={c.name} node={c} focus={focus} filtering={filtering} />
          ))}
          {node.operations.map((op) => (
            <OperationLink key={op.id} op={op} focus={focus} kind="operation" />
          ))}
        </ul>
      </details>
    </li>
  );
}

/** Tags (nested), webhooks and schemas; operations only render when their tag is open. */
export function SpecNav({ api, focus }: { api: Api; focus: SpecFocus }) {
  const [filter, setFilter] = useState("");
  const tree = useMemo(() => tagTree(api, filter.trim()), [api, filter]);
  const schemas = Object.keys(api.schemas);
  return (
    <nav
      aria-label="Spec contents"
      className="flex w-72 shrink-0 flex-col gap-2 overflow-y-auto border-r border-zinc-800 p-2 text-sm"
    >
      <input
        aria-label="Filter operations"
        placeholder="Filter operations"
        className="input"
        value={filter}
        onChange={(e) => {
          setFilter(e.target.value);
        }}
      />
      <button
        type="button"
        className="text-left text-zinc-300"
        onClick={() => {
          useSpecs.getState().focus({ kind: "overview" });
        }}
      >
        Overview
      </button>
      <ul aria-label="Operations by tag">
        {tree.map((n) => (
          <TagBranch key={n.name} node={n} focus={focus} filtering={filter.trim() !== ""} />
        ))}
      </ul>
      {api.webhooks.length > 0 && (
        <details>
          <summary className="cursor-pointer text-zinc-200">Webhooks</summary>
          <ul className="ml-2">
            {api.webhooks.map((op) => (
              <OperationLink key={op.id} op={op} focus={focus} kind="webhook" />
            ))}
          </ul>
        </details>
      )}
      {schemas.length > 0 && (
        <details>
          <summary className="cursor-pointer text-zinc-200">Schemas</summary>
          <ul className="ml-2">
            {schemas.map((name) => (
              <li key={name}>
                <button
                  type="button"
                  className="truncate px-1 text-left text-zinc-300 hover:bg-zinc-800/60"
                  onClick={() => {
                    useSpecs.getState().focus({ kind: "schema", name });
                  }}
                >
                  {name}
                </button>
              </li>
            ))}
          </ul>
        </details>
      )}
    </nav>
  );
}
