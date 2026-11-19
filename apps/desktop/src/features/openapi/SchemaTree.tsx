import { useState } from "react";

import { descriptionOf, refName, schemaChildren, schemaNotes, typeLabel } from "./browse";
import { useSpecs } from "./specsStore";

/** Deeper levels start closed, so cycles and large schemas cost nothing until opened. */
const OPEN_DEPTH = 1;

function TypeLink({ schema }: { schema: unknown }) {
  const ref = refName(schema);
  if (ref === null) return <span className="text-sky-300">{typeLabel(schema)}</span>;
  return (
    <button
      type="button"
      className="text-sky-300 underline"
      onClick={() => {
        useSpecs.getState().focus({ kind: "schema", name: ref });
      }}
    >
      {ref}
    </button>
  );
}

type RowProps = { name: string; schema: unknown; required: boolean; depth: number };

function Row({ name, schema, required, depth }: RowProps) {
  const children = refName(schema) === null ? schemaChildren(schema) : [];
  const [open, setOpen] = useState(depth < OPEN_DEPTH);
  const notes = schemaNotes(schema);
  const description = descriptionOf(schema);
  return (
    <li>
      <div className="flex flex-wrap items-baseline gap-2">
        {children.length > 0 ? (
          <button
            type="button"
            aria-expanded={open}
            aria-label={`${open ? "Collapse" : "Expand"} ${name}`}
            className="w-4 text-zinc-500"
            onClick={() => {
              setOpen(!open);
            }}
          >
            {open ? "▾" : "▸"}
          </button>
        ) : (
          <span className="w-4" />
        )}
        <span className="font-mono text-zinc-100">{name}</span>
        {required && <span className="text-xs text-red-400">required</span>}
        <TypeLink schema={schema} />
        {notes.length > 0 && <span className="text-xs text-zinc-500">{notes.join(" · ")}</span>}
      </div>
      {description && <p className="ml-6 text-xs text-zinc-400">{description}</p>}
      {open && children.length > 0 && (
        <ul className="ml-4 border-l border-zinc-800 pl-2">
          {children.map((c) => (
            <Row
              key={c.name}
              name={c.name}
              schema={c.schema}
              required={c.required}
              depth={depth + 1}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

/** A schema as an expandable tree; `$ref`s are links to the schema table. */
export function SchemaTree({ schema, label = "body" }: { schema: unknown; label?: string }) {
  return (
    <ul aria-label={`Schema of ${label}`} className="text-sm">
      <Row name={label} schema={schema} required={false} depth={0} />
    </ul>
  );
}
