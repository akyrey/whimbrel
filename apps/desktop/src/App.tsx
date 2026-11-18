import { CommandPalette, type PaletteItem } from "@roost/palette";
import { ShortcutsView } from "@roost/shortcuts";
import {
  Button,
  ConfirmDialog,
  EditorSettingsProvider,
  PanelBoundary,
  Toaster,
  resolveTheme,
  toast,
  useApplyResolvedTheme,
  usePrefersDark,
} from "@roost/ui";
import { useEffect, useMemo, useState } from "react";

import { onCollectionChanged, toAppError } from "./ipc";
import { openCollection } from "./features/collections/actions";
import { CollectionTree } from "./features/collections/CollectionTree";
import { useCollections } from "./features/collections/collectionsStore";
import { BodyEditor } from "./features/request/BodyEditor";
import { HeadersEditor } from "./features/request/HeadersEditor";
import { RequestBar, URL_INPUT_ID } from "./features/request/RequestBar";
import { ResponsePanel } from "./features/response/ResponsePanel";
import {
  SHORTCUT_GROUPS,
  type ShortcutId,
  matchShortcut,
  registry,
  shortcutLabel,
} from "./features/shortcuts/registry";
import { CurlImportDialog } from "./features/curl/CurlImportDialog";
import { useEnvironments } from "./features/environments/environmentsStore";
import { HistoryPanel } from "./features/history/HistoryPanel";
import { AuthPanel } from "./features/request/AuthPanel";
import { SettingsPanel } from "./features/request/SettingsPanel";
import { ExamplesBar } from "./features/examples/ExamplesBar";
import { cancelSend, confirmSend, sendTab, useSendConfirm } from "./features/send/sendFlow";
import { ConflictBar } from "./features/tabs/ConflictBar";
import type { Tab } from "./features/tabs/model";
import { TabStrip } from "./features/tabs/TabStrip";
import { useActiveTab, useTabs } from "./features/tabs/tabsStore";

type Overlay = "palette" | "shortcuts" | null;

const EDITOR_FONT = { family: "ui-monospace, SFMono-Regular, Menlo, monospace", size: 13 };

const activeId = () => useTabs.getState().activeId;

/** Saves the active tab; a scratch tab goes to the top of the first open collection. */
async function saveActive(): Promise<void> {
  const id = activeId();
  if (!id) return;
  const tab = useTabs.getState().tabs.find((t) => t.id === id);
  const firstRoot = Object.keys(useCollections.getState().views)[0];
  if (tab?.file === null && firstRoot === undefined) {
    toast.info("Open or create a collection to save this request in.");
    return;
  }
  try {
    await useTabs
      .getState()
      .save(id, firstRoot === undefined ? undefined : { root: firstRoot, path: "" });
    const root = useTabs.getState().tabs.find((t) => t.id === activeId())?.file?.root;
    if (root) await useCollections.getState().refresh(root);
  } catch (caught) {
    const error = toAppError(caught);
    toast.error(
      error.code === "conflict"
        ? "The file changed on disk since it was opened; it is shown again in a moment."
        : `Could not save: ${error.message}`,
    );
  }
}

function useShortcutActions(setOverlay: (overlay: Overlay) => void) {
  return useMemo(
    (): Record<ShortcutId, () => void> => ({
      palette: () => {
        setOverlay("palette");
      },
      shortcuts: () => {
        setOverlay("shortcuts");
      },
      "new-tab": () => {
        useTabs.getState().newScratch();
      },
      "close-tab": () => {
        const id = activeId();
        if (id) useTabs.getState().close(id);
      },
      save: () => void saveActive(),
      send: () => {
        const id = activeId();
        if (id) sendTab(id);
      },
      cancel: () => {
        const id = activeId();
        if (id) void useTabs.getState().cancel(id);
      },
      "focus-url": () => {
        document.getElementById(URL_INPUT_ID)?.focus();
      },
    }),
    [setOverlay],
  );
}

/** "Send with <example>" for the active tab's examples. */
function exampleItems(): PaletteItem[] {
  const tab = useTabs.getState().tabs.find((t) => t.id === activeId());
  if (!tab) return [];
  return tab.draft.examples.map((example) => ({
    id: `example:${example.id}`,
    group: "Examples",
    label: `Send ${tab.draft.name} › ${example.name}`,
    run: () => {
      useTabs.getState().pickExample(tab.id, example.id);
      sendTab(tab.id);
    },
  }));
}

function paletteItems(actions: Record<ShortcutId, () => void>): PaletteItem[] {
  const item = (id: ShortcutId, label: string): PaletteItem => ({
    id,
    group: "Actions",
    label,
    hint: shortcutLabel(id),
    run: actions[id],
  });
  return [
    item("send", "Send the request"),
    item("save", "Save the request"),
    item("new-tab", "New request tab"),
    item("cancel", "Cancel the request"),
    item("focus-url", "Go to the URL"),
    item("shortcuts", "Keyboard shortcuts"),
    {
      id: "open-collection",
      group: "Collections",
      label: "Open a collection",
      run: () => void openCollection(),
    },
    ...exampleItems(),
  ];
}

/** Reopens what was open at the last quit, and follows changes on disk. */
function useCollectionsLifecycle() {
  useEffect(() => {
    void (async () => {
      const failed = await useCollections.getState().restore();
      if (failed.length > 0) toast.error(`Could not reopen: ${failed.join(", ")}`);
      await useTabs.getState().restore();
    })();
    const stop = onCollectionChanged((changed) => {
      void useCollections.getState().refresh(changed.root);
      if (changed.paths.some((p) => p.startsWith("environments/"))) {
        void useEnvironments.getState().load(changed.root);
      }
      void useTabs.getState().changedOnDisk(changed.root, changed.paths);
    });
    return () => {
      void stop.then((unlisten) => {
        unlisten();
      });
    };
  }, []);
}

function RequestView({ tab }: { tab: Tab }) {
  return (
    <>
      <section aria-label="Request" className="flex flex-col gap-3 border-b border-zinc-800 p-4">
        <ConflictBar tab={tab} />
        <RequestBar tab={tab} />
        <ExamplesBar tab={tab} />
        <AuthPanel tab={tab} />
        <SettingsPanel tab={tab} />
        <PanelBoundary name="Headers">
          <HeadersEditor tab={tab} />
        </PanelBoundary>
        <div className="flex h-56 flex-col">
          <PanelBoundary name="Body">
            <BodyEditor tab={tab} />
          </PanelBoundary>
        </div>
      </section>
      <PanelBoundary name="Response">
        <ResponsePanel status={tab.status} />
      </PanelBoundary>
    </>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 text-sm text-zinc-400">
      <p>Open a request from a collection, or start a new one.</p>
      <Button onClick={useTabs.getState().newScratch}>New request</Button>
    </div>
  );
}

type Side = "collections" | "history";

/** Collections or history, and the cURL import. */
function Sidebar() {
  const [side, setSide] = useState<Side>("collections");
  const [importing, setImporting] = useState(false);
  return (
    <>
      <div
        role="tablist"
        aria-label="Sidebar"
        className="flex gap-3 border-b border-zinc-800 px-2 pt-2 text-sm"
      >
        {(["collections", "history"] as const).map((s) => (
          <button
            key={s}
            type="button"
            role="tab"
            aria-selected={side === s}
            className={side === s ? "text-zinc-100 underline" : "text-zinc-400"}
            onClick={() => {
              setSide(s);
            }}
          >
            {s === "collections" ? "Collections" : "History"}
          </button>
        ))}
        <button
          type="button"
          className="ml-auto text-xs text-zinc-400 hover:text-zinc-100"
          onClick={() => {
            setImporting(true);
          }}
        >
          Import cURL…
        </button>
      </div>
      <PanelBoundary name={side === "collections" ? "Collections" : "History"}>
        {side === "collections" ? <CollectionTree /> : <HistoryPanel />}
      </PanelBoundary>
      {importing && (
        <CurlImportDialog
          onClose={() => {
            setImporting(false);
          }}
        />
      )}
    </>
  );
}

/** Asks before sending with a protected environment (FR-4.6). */
function SendConfirm() {
  const pending = useSendConfirm((s) => s.pending);
  if (!pending) return null;
  return (
    <ConfirmDialog
      title="Send to a protected environment?"
      message={`${pending.environment} is protected. Send the request anyway?`}
      confirmLabel="Send"
      onCancel={cancelSend}
      onConfirm={confirmSend}
    />
  );
}

export function App() {
  const [overlay, setOverlay] = useState<Overlay>(null);
  const theme = resolveTheme("system", usePrefersDark());
  useApplyResolvedTheme(theme);
  const actions = useShortcutActions(setOverlay);
  const editorSettings = useMemo(() => ({ theme, font: EDITOR_FONT }), [theme]);
  const tab = useActiveTab();
  useCollectionsLifecycle();

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const id = matchShortcut(event);
      if (!id) return;
      event.preventDefault();
      actions[id]();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [actions]);

  const close = () => {
    setOverlay(null);
  };

  return (
    <EditorSettingsProvider settings={editorSettings}>
      <div className="flex h-full">
        <aside className="flex w-64 shrink-0 flex-col border-r border-zinc-800 bg-zinc-900">
          <Sidebar />
        </aside>
        <main className="flex min-w-0 flex-1 flex-col">
          <TabStrip />
          {tab ? <RequestView key={tab.id} tab={tab} /> : <EmptyState />}
        </main>
      </div>
      {overlay === "palette" && <CommandPalette commands={paletteItems(actions)} onClose={close} />}
      {overlay === "shortcuts" && (
        <ShortcutsView registry={registry} groups={SHORTCUT_GROUPS} onClose={close} />
      )}
      <SendConfirm />
      <Toaster />
    </EditorSettingsProvider>
  );
}
