import { useState } from "react";

import { AuthDialog } from "../auth/AuthDialog";
import { AuthEditor } from "../auth/AuthEditor";
import { OAuthPanel } from "../auth/OAuthPanel";
import { type InheritSource, inheritedFrom } from "../auth/inheritance";
import { useCollections } from "../collections/collectionsStore";
import type { FileRef, Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";

/** The folders and collection an inheriting request takes its auth from, each one editable. */
function InheritedAuth({ file }: { file: FileRef }) {
  const collection = useCollections((s) => s.views[file.root]?.collection);
  const [editing, setEditing] = useState<InheritSource | null>(null);
  if (!collection) return null;
  return (
    <div role="group" aria-label="Inherited auth" className="flex flex-wrap items-center gap-1">
      <span className="text-xs text-zinc-400">
        Uses the nearest auth set on, in order (click to edit):
      </span>
      {inheritedFrom(collection, file.path).map((source) => (
        <button
          key={source.folder}
          type="button"
          className="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-200 hover:bg-zinc-700"
          onClick={() => {
            setEditing(source);
          }}
        >
          {source.name}
        </button>
      ))}
      {editing && (
        <AuthDialog
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

/** The request's own auth, and its OAuth 2 token when that is what applies. */
export function AuthPanel({ tab }: { tab: Tab }) {
  return (
    <div className="flex flex-col gap-2">
      <AuthEditor
        auth={tab.draft.auth}
        allowInherit
        onChange={(auth) => {
          useTabs.getState().edit(tab.id, (d) => ({ ...d, auth }));
        }}
      />
      {tab.draft.auth.type === "inherit" && tab.file && <InheritedAuth file={tab.file} />}
      <OAuthPanel tab={tab} />
    </div>
  );
}
