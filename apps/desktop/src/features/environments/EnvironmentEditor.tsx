import { Button, ConfirmDialog, Modal, toast } from "@roost/ui";
import { useCallback, useEffect, useState } from "react";

import {
  type EnvironmentView,
  type SecretName,
  collectionVariablesRead,
  environmentCreate,
  environmentDelete,
  environmentRead,
  environmentSave,
  secretClear,
  secretSet,
  toAppError,
  variablesSave,
} from "../../ipc";
import { NameDialog } from "../collections/NameDialog";
import { NO_ENVIRONMENTS, useEnvironments } from "./environmentsStore";
import { KeyValueTable } from "./KeyValueTable";
import { type Row, toMap, toRows } from "./rows";
import { SecretsEditor } from "./SecretsEditor";

/** `null` selects the collection's own variables. */
type Selected = string | null;

type Draft = {
  name: string;
  protected: boolean;
  variables: Row[];
  secrets: SecretName[];
  local: Row[];
};

const draftOf = (view: EnvironmentView): Draft => ({
  name: view.environment.name,
  protected: view.environment.protected,
  variables: toRows(view.environment.variables),
  secrets: view.environment.secrets,
  local: toRows(view.local),
});

async function attempt(what: string, run: () => Promise<void>): Promise<void> {
  try {
    await run();
  } catch (caught) {
    toast.error(`Could not ${what}: ${toAppError(caught).message}`);
  }
}

function CollectionVariables({ root }: { root: string }) {
  const [rows, setRows] = useState<Row[] | null>(null);
  useEffect(() => {
    void collectionVariablesRead(root).then((vars) => {
      setRows(toRows(vars));
    });
  }, [root]);
  if (rows === null) return null;
  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs text-zinc-400">
        Used in every environment, after the environment's own.
      </p>
      <KeyValueTable label="Collection variables" rows={rows} onChange={setRows} />
      <div>
        <Button
          variant="primary"
          onClick={() =>
            void attempt("save the variables", async () => {
              await variablesSave(root, null, toMap(rows));
              useEnvironments.getState().changed();
              toast.success("Saved");
            })
          }
        >
          Save
        </Button>
      </div>
    </div>
  );
}

function EnvironmentForm({
  root,
  id,
  onDeleted,
}: {
  root: string;
  id: string;
  onDeleted: () => void;
}) {
  const [view, setView] = useState<EnvironmentView | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const reload = useCallback(async () => {
    const read = await environmentRead(root, id);
    setView(read);
    setDraft(draftOf(read));
  }, [root, id]);
  useEffect(() => {
    void attempt("read the environment", reload);
  }, [reload]);
  if (!view || !draft) return null;

  const save = () =>
    attempt("save the environment", async () => {
      const environment = {
        name: draft.name,
        protected: draft.protected,
        variables: toMap(draft.variables),
        secrets: draft.secrets.filter((s) => s.name.trim() !== ""),
      };
      await environmentSave(root, id, environment, view.hash);
      await variablesSave(root, id, toMap(draft.local));
      await useEnvironments.getState().load(root);
      await reload();
      toast.success("Saved");
    });

  return (
    <div className="flex flex-col gap-3">
      <label className="flex items-center gap-2 text-sm">
        Name
        <input
          aria-label="Environment name"
          className="input flex-1"
          value={draft.name}
          onChange={(e) => {
            setDraft({ ...draft, name: e.target.value });
          }}
        />
      </label>
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={draft.protected}
          onChange={(e) => {
            setDraft({ ...draft, protected: e.target.checked });
          }}
        />
        Protected: ask before sending
      </label>
      <KeyValueTable
        label="Variables"
        rows={draft.variables}
        onChange={(variables) => {
          setDraft({ ...draft, variables });
        }}
      />
      <SecretsEditor
        secrets={draft.secrets}
        states={view.secrets}
        onChange={(secrets) => {
          setDraft({ ...draft, secrets });
        }}
        onSet={(name, value) =>
          attempt("store the secret", async () => {
            await secretSet(root, id, name, value);
            useEnvironments.getState().changed();
            await reload();
          })
        }
        onClear={(name) =>
          attempt("clear the secret", async () => {
            await secretClear(root, id, name);
            useEnvironments.getState().changed();
            await reload();
          })
        }
      />
      <KeyValueTable
        label="My overrides (not committed)"
        rows={draft.local}
        onChange={(local) => {
          setDraft({ ...draft, local });
        }}
      />
      <div className="flex gap-2">
        <Button variant="primary" onClick={() => void save()}>
          Save
        </Button>
        <Button
          variant="danger"
          onClick={() => {
            setConfirmDelete(true);
          }}
        >
          Delete
        </Button>
      </div>
      {confirmDelete && (
        <ConfirmDialog
          title="Delete environment?"
          message={`"${draft.name}" and its secret values are deleted.`}
          confirmLabel="Delete"
          onCancel={() => {
            setConfirmDelete(false);
          }}
          onConfirm={() => {
            setConfirmDelete(false);
            void attempt("delete the environment", async () => {
              await environmentDelete(root, id);
              await useEnvironments.getState().load(root);
              onDeleted();
            });
          }}
        />
      )}
    </div>
  );
}

/** Environments, secrets and variables of one collection (Phase 3, task 3.6). */
export function EnvironmentEditor({ root, onClose }: { root: string; onClose: () => void }) {
  const entries = useEnvironments((s) => s.entries[root] ?? NO_ENVIRONMENTS);
  const [selected, setSelected] = useState<Selected>(entries[0]?.id ?? null);
  const [naming, setNaming] = useState(false);

  const create = (name: string) =>
    attempt("create the environment", async () => {
      const id = await environmentCreate(root, {
        name,
        protected: false,
        variables: {},
        secrets: [],
      });
      await useEnvironments.getState().load(root);
      setSelected(id);
    });

  return (
    <Modal
      title="Environments"
      size="wide"
      onClose={onClose}
      footer={<Button onClick={onClose}>Close</Button>}
    >
      <div className="flex max-h-[65vh] gap-4 overflow-y-auto">
        <ul aria-label="Environments" className="flex w-40 shrink-0 flex-col gap-1 text-sm">
          {entries.map((entry) => (
            <li key={entry.id}>
              <button
                type="button"
                aria-current={selected === entry.id}
                disabled={entry.error !== null}
                title={entry.error ?? undefined}
                className={`w-full truncate rounded px-2 py-1 text-left ${selected === entry.id ? "bg-zinc-800 text-zinc-100" : "text-zinc-300"} disabled:text-rose-400`}
                onClick={() => {
                  setSelected(entry.id);
                }}
              >
                {entry.name ?? `⚠ ${entry.id}`}
                {entry.protected && " 🔒"}
              </button>
            </li>
          ))}
          <li>
            <button
              type="button"
              aria-current={selected === null}
              className={`w-full rounded px-2 py-1 text-left italic ${selected === null ? "bg-zinc-800" : "text-zinc-400"}`}
              onClick={() => {
                setSelected(null);
              }}
            >
              Collection variables
            </button>
          </li>
          <li>
            <Button
              onClick={() => {
                setNaming(true);
              }}
            >
              New environment
            </Button>
          </li>
        </ul>
        <div className="min-w-0 flex-1">
          {selected === null ? (
            <CollectionVariables root={root} />
          ) : (
            <EnvironmentForm
              key={selected}
              root={root}
              id={selected}
              onDeleted={() => {
                setSelected(null);
              }}
            />
          )}
        </div>
      </div>
      {naming && (
        <NameDialog
          title="New environment"
          label="Name"
          initial=""
          confirmLabel="Create"
          onCancel={() => {
            setNaming(false);
          }}
          onSubmit={(name) => {
            setNaming(false);
            void create(name);
          }}
        />
      )}
    </Modal>
  );
}
