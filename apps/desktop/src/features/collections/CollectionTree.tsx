import { Button, ConfirmDialog, ContextMenu, type ContextMenuItem } from "@roost/ui";
import { type MouseEvent, useEffect, useState } from "react";

import type { CollectionView, Node } from "../../ipc";
import { AuthDialog } from "../auth/AuthDialog";
import { TlsDialog } from "../auth/TlsDialog";
import { EnvironmentEditor } from "../environments/EnvironmentEditor";
import { NO_ENVIRONMENTS, useEnvironments } from "../environments/environmentsStore";
import { fileTabId } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import {
  chooseCollectionFolder,
  closeCollection,
  createCollection,
  deleteNode,
  duplicateRequest,
  newFolder,
  newRequest,
  openCollection,
  renameRequest,
  exportHttp,
  moveNode,
  renameFolder,
} from "./actions";
import { useCollections } from "./collectionsStore";
import { filterNodes } from "./filter";
import { NameDialog } from "./NameDialog";
import { useRunner } from "../runner/runnerStore";

type Menu = { x: number; y: number; label: string; items: ContextMenuItem[] };

type Dialog =
  | { kind: "auth"; root: string; folder: string; name: string }
  | { kind: "tls"; root: string }
  | { kind: "name"; title: string; initial: string; confirm: string; run: (name: string) => void }
  | { kind: "confirm"; title: string; message: string; run: () => void };

type Ui = {
  menu: (event: MouseEvent, label: string, items: ContextMenuItem[]) => void;
  ask: (dialog: Dialog) => void;
  /** The filter text; while set, folders are shown open. */
  filter: string;
};

/** The tree's filter box, for the shortcut that focuses it. */
export const TREE_FILTER_ID = "tree-filter";

const METHOD_CLASS: Record<string, string> = {
  GET: "text-emerald-400",
  POST: "text-amber-300",
  PUT: "text-sky-300",
  PATCH: "text-violet-300",
  DELETE: "text-rose-400",
};

/** Opens the request with the example picked for sending. */
async function openExample(root: string, path: string, exampleId: string) {
  await useTabs.getState().openFile({ root, path });
  useTabs.getState().pickExample(fileTabId({ root, path }), exampleId);
}

function containerItems(ui: Ui, root: string, folder: string, name: string): ContextMenuItem[] {
  return [
    {
      label: "Run…",
      onSelect: () => {
        useRunner.getState().open({ root, folder, name });
      },
    },
    { label: "Export as .http…", onSelect: () => void exportHttp(root, folder, name) },
    {
      label: "New request",
      onSelect: () => {
        ui.ask({
          kind: "name",
          title: "New request",
          initial: "New request",
          confirm: "Create",
          run: (name) => void newRequest(root, folder, name),
        });
      },
    },
    {
      label: "New folder",
      onSelect: () => {
        ui.ask({
          kind: "name",
          title: "New folder",
          initial: "",
          confirm: "Create",
          run: (name) => void newFolder(root, folder, name),
        });
      },
    },
  ];
}

function deleteItem(ui: Ui, root: string, path: string, name: string, folder: boolean) {
  return {
    label: "Delete",
    destructive: true,
    onSelect: () => {
      ui.ask({
        kind: "confirm",
        title: folder ? "Delete folder?" : "Delete request?",
        message: folder
          ? `"${name}" and everything in it are deleted from disk.`
          : `"${name}" is deleted from disk.`,
        run: () => void deleteNode(root, path),
      });
    },
  };
}

type ItemProps = {
  node: Node;
  root: string;
  ui: Ui;
  depth: number;
  siblings: Node[];
  index: number;
};

/** "Move up" and "Move down", where there is somewhere to go. */
function moveItems(root: string, siblings: Node[], index: number): ContextMenuItem[] {
  const items: ContextMenuItem[] = [];
  if (index > 0)
    items.push({ label: "Move up", onSelect: () => void moveNode(root, siblings, index, -1) });
  if (index < siblings.length - 1) {
    items.push({ label: "Move down", onSelect: () => void moveNode(root, siblings, index, 1) });
  }
  return items;
}

function NodeItem({ node, root, ui, depth, siblings, index }: ItemProps) {
  const [opened, setOpen] = useState(true);
  const open = opened || ui.filter.trim() !== "";
  const indent = { paddingLeft: `${String(depth * 12 + 8)}px` };
  if (node.kind === "broken") {
    return (
      <li
        className="truncate py-0.5 text-sm text-rose-400"
        style={indent}
        title={node.error}
        aria-label={`${node.path} cannot be read: ${node.error}`}
      >
        ⚠ {node.path}
      </li>
    );
  }
  if (node.kind === "request") {
    const items: ContextMenuItem[] = [
      {
        label: "Rename",
        onSelect: () => {
          ui.ask({
            kind: "name",
            title: "Rename request",
            initial: node.name,
            confirm: "Rename",
            run: (name) => void renameRequest(root, node.path, name),
          });
        },
      },
      { label: "Duplicate", onSelect: () => void duplicateRequest(root, node.path) },
      ...moveItems(root, siblings, index),
      deleteItem(ui, root, node.path, node.name, false),
    ];
    return (
      <li>
        <button
          type="button"
          className="flex w-full items-center gap-2 py-0.5 text-left text-sm text-zinc-300 hover:bg-zinc-800"
          style={indent}
          onClick={() => void useTabs.getState().openFile({ root, path: node.path })}
          onContextMenu={(e) => {
            ui.menu(e, `Actions for ${node.name}`, items);
          }}
        >
          <span
            className={`w-12 shrink-0 font-mono text-[10px] ${METHOD_CLASS[node.method] ?? "text-zinc-400"}`}
          >
            {node.method}
          </span>
          <span className="truncate">{node.name}</span>
        </button>
        {node.examples.length > 0 && (
          <ul aria-label={`Examples of ${node.name}`}>
            {node.examples.map((example) => (
              <li key={example.id}>
                <button
                  type="button"
                  className="w-full truncate py-0.5 text-left text-xs text-zinc-400 hover:bg-zinc-800"
                  style={{ paddingLeft: `${String(depth * 12 + 64)}px` }}
                  onClick={() => void openExample(root, node.path, example.id)}
                >
                  ↳ {example.name}
                </button>
              </li>
            ))}
          </ul>
        )}
      </li>
    );
  }
  const items = [
    ...containerItems(ui, root, node.path, node.name),
    {
      label: "Rename",
      onSelect: () => {
        ui.ask({
          kind: "name",
          title: "Rename folder",
          initial: node.name,
          confirm: "Rename",
          run: (name) => void renameFolder(root, node.path, name),
        });
      },
    },
    ...moveItems(root, siblings, index),
    deleteItem(ui, root, node.path, node.name, true),
  ];
  return (
    <li>
      <button
        type="button"
        aria-expanded={open}
        className="flex w-full items-center gap-1 py-0.5 text-left text-sm text-zinc-200 hover:bg-zinc-800"
        style={indent}
        onClick={() => {
          setOpen(!opened);
        }}
        onContextMenu={(e) => {
          ui.menu(e, `Actions for ${node.name}`, items);
        }}
      >
        <span aria-hidden="true">{open ? "▾" : "▸"}</span>
        {node.name}
      </button>
      {open && <Nodes nodes={node.children} root={root} ui={ui} depth={depth + 1} />}
    </li>
  );
}

function Nodes({ nodes, root, ui, depth }: { nodes: Node[]; root: string; ui: Ui; depth: number }) {
  return (
    <ul>
      {nodes.map((node, index) => (
        <NodeItem
          key={node.path}
          node={node}
          root={root}
          ui={ui}
          depth={depth}
          siblings={nodes}
          index={index}
        />
      ))}
    </ul>
  );
}

function EnvironmentPicker({ root }: { root: string }) {
  const entries = useEnvironments((s) => s.entries[root] ?? NO_ENVIRONMENTS);
  const active = useEnvironments((s) => s.active[root] ?? null);
  const [editing, setEditing] = useState(false);
  useEffect(() => {
    void useEnvironments.getState().load(root);
  }, [root]);
  return (
    <div className="flex items-center gap-1 px-2 pb-1">
      <select
        aria-label="Environment"
        className="input min-w-0 flex-1 py-0 text-xs"
        value={active ?? ""}
        onChange={(e) => {
          useEnvironments.getState().setActive(root, e.target.value === "" ? null : e.target.value);
        }}
      >
        <option value="">No environment</option>
        {entries
          .filter((entry) => entry.error === null)
          .map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.name}
              {entry.protected ? " 🔒" : ""}
            </option>
          ))}
      </select>
      <button
        type="button"
        aria-label="Edit environments"
        title="Environments, secrets and variables"
        className="px-1 text-xs text-zinc-400 hover:text-zinc-100"
        onClick={() => {
          setEditing(true);
        }}
      >
        ⚙
      </button>
      {editing && (
        <EnvironmentEditor
          root={root}
          onClose={() => {
            setEditing(false);
          }}
        />
      )}
    </div>
  );
}

function CollectionItem({ view, ui }: { view: CollectionView; ui: Ui }) {
  const shown = filterNodes(view.collection.children, ui.filter);
  const items = [
    ...containerItems(ui, view.root, "", view.collection.name),
    {
      label: "Auth…",
      onSelect: () => {
        ui.ask({ kind: "auth", root: view.root, folder: "", name: view.collection.name });
      },
    },
    {
      label: "TLS…",
      onSelect: () => {
        ui.ask({ kind: "tls", root: view.root });
      },
    },
    { label: "Close collection", onSelect: () => void closeCollection(view.root) },
  ];
  return (
    <section aria-label={view.collection.name} className="py-1">
      <h2
        className="truncate px-2 py-1 text-xs font-semibold uppercase tracking-wide text-zinc-400"
        title={view.root}
        onContextMenu={(e) => {
          ui.menu(e, `Actions for ${view.collection.name}`, items);
        }}
      >
        {view.collection.name}
      </h2>
      <EnvironmentPicker root={view.root} />
      {view.collection.children.length === 0 ? (
        <p className="px-2 text-xs text-zinc-500">Empty. Right-click the name to add a request.</p>
      ) : shown.length === 0 ? (
        <p className="px-2 text-xs text-zinc-500">No matches.</p>
      ) : (
        <Nodes nodes={shown} root={view.root} ui={ui} depth={0} />
      )}
    </section>
  );
}

/** The open collections, with menus for their requests and folders (Phase 2, task 2.7). */
export function CollectionTree() {
  const views = useCollections((s) => s.views);
  const [menu, setMenu] = useState<Menu | null>(null);
  const [dialog, setDialog] = useState<Dialog | null>(null);
  const [filter, setFilter] = useState("");
  const ui: Ui = {
    filter,
    menu: (event, label, items) => {
      event.preventDefault();
      setMenu({ x: event.clientX, y: event.clientY, label, items });
    },
    ask: setDialog,
  };
  const startCollection = async () => {
    const root = await chooseCollectionFolder();
    if (!root) return;
    ui.ask({
      kind: "name",
      title: "New collection",
      initial: "",
      confirm: "Create",
      run: (name) => void createCollection(root, name),
    });
  };

  return (
    <nav aria-label="Collections" className="flex min-h-0 flex-1 flex-col">
      <div className="flex gap-1 p-2">
        <Button onClick={() => void openCollection()}>Open…</Button>
        <Button onClick={() => void startCollection()}>New collection…</Button>
      </div>
      <input
        id={TREE_FILTER_ID}
        type="search"
        aria-label="Filter requests"
        placeholder="Filter requests"
        className="input mx-2 mb-1 py-0.5 text-xs"
        value={filter}
        spellCheck={false}
        onChange={(e) => {
          setFilter(e.target.value);
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") setFilter("");
        }}
      />
      <div className="min-h-0 flex-1 overflow-y-auto">
        {Object.values(views).map((view) => (
          <CollectionItem key={view.root} view={view} ui={ui} />
        ))}
      </div>
      {menu && (
        <ContextMenu
          label={menu.label}
          x={menu.x}
          y={menu.y}
          items={menu.items}
          onClose={() => {
            setMenu(null);
          }}
        />
      )}
      {dialog?.kind === "name" && (
        <NameDialog
          title={dialog.title}
          label="Name"
          initial={dialog.initial}
          confirmLabel={dialog.confirm}
          onCancel={() => {
            setDialog(null);
          }}
          onSubmit={(name) => {
            setDialog(null);
            dialog.run(name);
          }}
        />
      )}
      {dialog?.kind === "auth" && (
        <AuthDialog
          root={dialog.root}
          folder={dialog.folder}
          name={dialog.name}
          onClose={() => {
            setDialog(null);
          }}
        />
      )}
      {dialog?.kind === "tls" && (
        <TlsDialog
          root={dialog.root}
          onClose={() => {
            setDialog(null);
          }}
        />
      )}
      {dialog?.kind === "confirm" && (
        <ConfirmDialog
          title={dialog.title}
          message={dialog.message}
          confirmLabel="Delete"
          onCancel={() => {
            setDialog(null);
          }}
          onConfirm={() => {
            setDialog(null);
            dialog.run();
          }}
        />
      )}
    </nav>
  );
}
