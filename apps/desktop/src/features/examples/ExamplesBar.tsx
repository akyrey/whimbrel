import { Button, ConfirmDialog, toast } from "@roost/ui";
import { useState } from "react";

import { type Example, exampleApply, exampleOverlay, toAppError } from "../../ipc";
import { useCollections } from "../collections/collectionsStore";
import { NameDialog } from "../collections/NameDialog";
import { type Tab, pickedExample } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";

type Dialog = "new" | "delete" | null;

async function attempt(what: string, run: () => Promise<void>): Promise<void> {
  try {
    await run();
  } catch (caught) {
    toast.error(`Could not ${what}: ${toAppError(caught).message}`);
  }
}

/** Records the tab's edits as `example` (new or replacing one with its id), and puts the
 * request back as saved: the edits now live in the example. Saves when the tab has a file. */
async function storeExample(tab: Tab, id: string, name: string): Promise<void> {
  const example = await exampleOverlay(tab.saved.request, tab.draft.request, id, name);
  const others = tab.draft.examples.filter((e) => e.id !== id);
  const kept = tab.draft.examples.find((e) => e.id === id);
  const placed: Example = { ...example, order: kept?.order ?? (others.length + 1) * 10 };
  const { edit, pickExample } = useTabs.getState();
  edit(tab.id, (d) => ({ ...d, request: tab.saved.request, examples: [...others, placed] }));
  pickExample(tab.id, id);
  await persist(tab);
}

/** Saves the tab when it has a file, and shows the change in the tree. */
async function persist(tab: Tab): Promise<void> {
  if (!tab.file) return;
  await useTabs.getState().save(tab.id);
  await useCollections.getState().refresh(tab.file.root);
}

/** Saved examples of the request: pick one to send with, apply, save, update, delete (FR-2.8). */
export function ExamplesBar({ tab }: { tab: Tab }) {
  const [dialog, setDialog] = useState<Dialog>(null);
  const picked = pickedExample(tab);
  const { pickExample, edit } = useTabs.getState();

  return (
    <div className="flex items-center gap-2 text-sm">
      <select
        aria-label="Example"
        className="input w-48"
        value={picked?.id ?? ""}
        onChange={(e) => {
          pickExample(tab.id, e.target.value === "" ? null : e.target.value);
        }}
      >
        <option value="">No example</option>
        {tab.draft.examples.map((example) => (
          <option key={example.id} value={example.id}>
            {example.name}
          </option>
        ))}
      </select>
      <Button
        onClick={() => {
          setDialog("new");
        }}
      >
        Save as example…
      </Button>
      {picked && (
        <>
          <Button
            onClick={() =>
              void attempt("apply the example", async () => {
                const request = await exampleApply(tab.draft.request, picked);
                edit(tab.id, (d) => ({ ...d, request }));
              })
            }
          >
            Apply to editor
          </Button>
          <Button
            onClick={() =>
              void attempt("update the example", () => storeExample(tab, picked.id, picked.name))
            }
          >
            Update example
          </Button>
          <Button
            onClick={() => {
              setDialog("delete");
            }}
          >
            Delete example
          </Button>
        </>
      )}
      {dialog === "new" && (
        <NameDialog
          title="Save as example"
          label="Example name"
          initial=""
          confirmLabel="Save"
          onCancel={() => {
            setDialog(null);
          }}
          onSubmit={(name) => {
            setDialog(null);
            void attempt("save the example", () => storeExample(tab, crypto.randomUUID(), name));
          }}
        />
      )}
      {dialog === "delete" && picked && (
        <ConfirmDialog
          title="Delete example?"
          message={`"${picked.name}" is removed from the request.`}
          confirmLabel="Delete"
          onCancel={() => {
            setDialog(null);
          }}
          onConfirm={() => {
            setDialog(null);
            pickExample(tab.id, null);
            edit(tab.id, (d) => ({ ...d, examples: d.examples.filter((e) => e.id !== picked.id) }));
            void attempt("delete the example", () => persist(tab));
          }}
        />
      )}
    </div>
  );
}
