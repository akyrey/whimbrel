import { toast } from "@roost/ui";
import { useEffect, useState } from "react";

import { type Header, nodeHeadersRead, toAppError } from "../../ipc";
import { type InheritSource, inheritedFrom } from "../auth/inheritance";
import { useCollections } from "../collections/collectionsStore";
import type { FileRef } from "../tabs/model";
import { HeadersDialog } from "./HeadersDialog";
import { type HeadersLevel, inheritedRows } from "./inherit";
import { useHeadersVersion } from "./headersVersion";

/** The headers of each folder around `file` and of its collection, read again on a save. */
function useLevels(file: FileRef): HeadersLevel[] {
  const collection = useCollections((s) => s.views[file.root]?.collection);
  const version = useHeadersVersion((s) => s.version);
  const [levels, setLevels] = useState<HeadersLevel[]>([]);
  useEffect(() => {
    if (!collection) return;
    let current = true;
    const sources = inheritedFrom(collection, file.path);
    Promise.all(
      sources.map(async (source) => ({
        source,
        headers: await nodeHeadersRead(file.root, source.folder),
      })),
    ).then(
      (read) => {
        if (current) setLevels(read);
      },
      (caught: unknown) => {
        toast.error(`Could not read the inherited headers: ${toAppError(caught).message}`);
      },
    );
    return () => {
      current = false;
    };
  }, [collection, file.root, file.path, version]);
  return levels;
}

/** What the request receives from its folders and collection, read-only; each source opens its
 * headers for editing. */
export function InheritedHeaders({ file, own }: { file: FileRef; own: Header[] }) {
  const rows = inheritedRows(useLevels(file), own);
  const [editing, setEditing] = useState<InheritSource | null>(null);
  if (rows.length === 0) return null;
  return (
    <div className="mt-3 flex flex-col gap-1 text-sm">
      <span className="text-xs text-zinc-400">
        Inherited (a header of the same name above replaces one):
      </span>
      <ul aria-label="Inherited headers" className="flex flex-col gap-1">
        {rows.map(({ header, source, overridden }, index) => (
          <li
            key={index}
            className={`flex items-center gap-2 font-mono ${header.enabled ? "" : "opacity-50"}`}
          >
            <span className={`w-56 truncate ${overridden ? "line-through text-zinc-500" : ""}`}>
              {header.name}
            </span>
            <span className="min-w-0 flex-1 truncate text-zinc-400">{header.value}</span>
            <button
              type="button"
              className="rounded bg-zinc-800 px-2 py-0.5 font-sans text-xs text-zinc-200 hover:bg-zinc-700"
              onClick={() => {
                setEditing(source);
              }}
            >
              {source.name}
            </button>
          </li>
        ))}
      </ul>
      {editing && (
        <HeadersDialog
          root={file.root}
          folder={editing.folder}
          name={editing.name}
          onClose={() => {
            setEditing(null);
          }}
        />
      )}
    </div>
  );
}
