import { CommandPalette, type PaletteItem } from "@roost/palette";
import { ShortcutsView } from "@roost/shortcuts";
import {
  ConfirmDialog,
  EditorSettingsProvider,
  PanelBoundary,
  SidebarResizer,
  Toaster,
  resolveTheme,
  toast,
  useApplyResolvedTheme,
  usePrefersDark,
} from "@roost/ui";
import { useEffect, useMemo, useRef, useState } from "react";

import { onCollectionChanged, toAppError } from "./ipc";
import { CollectionTree, TREE_FILTER_ID } from "./features/collections/CollectionTree";
import { useCollections } from "./features/collections/collectionsStore";
import { RequestBar, URL_INPUT_ID } from "./features/request/RequestBar";
import { RequestSections } from "./features/request/RequestSections";
import { SplitResizer } from "./features/layout/SplitResizer";
import { SIDEBAR_WIDTH, currentShare, useLayout } from "./features/layout/layoutStore";
import { useSidebar } from "./features/layout/sidebarStore";
import { beautifyBody } from "./features/request/bodyActions";
import { ResponsePanel } from "./features/response/ResponsePanel";
import {
  SHORTCUT_GROUPS,
  type ShortcutId,
  matchShortcut,
  registry,
  shortcutLabel,
} from "./features/shortcuts/registry";
import { SchemaCheck } from "./features/openapi/SchemaCheck";
import { ShowInSpec } from "./features/openapi/ShowInSpec";
import { SpecBrowser } from "./features/openapi/SpecBrowser";
import { SpecList } from "./features/openapi/SpecList";
import { syncSpecs, useSpecs } from "./features/openapi/specsStore";
import { useEnvironments } from "./features/environments/environmentsStore";
import { HistoryPanel } from "./features/history/HistoryPanel";
import { UsedVariables } from "./features/variables/UsedVariables";
import { VariablesContext, useScopeVariables } from "./features/variables/scope";
import { RunnerDialog } from "./features/runner/RunnerDialog";
import { ExamplesBar } from "./features/examples/ExamplesBar";
import { AppDialogs } from "./features/shell/AppDialogs";
import { EmptyState } from "./features/shell/EmptyState";
import { UpdatePrompt } from "./features/updates/UpdatePrompt";
import { useUpdates } from "./features/updates/updateStore";
import { useDialogs } from "./features/shell/dialogsStore";
import { appItems } from "./features/shell/paletteItems";
import { useSettings } from "./features/settings/settingsStore";
import { cancelSend, confirmSend, sendTab, useSendConfirm } from "./features/send/sendFlow";
import { ConflictBar } from "./features/tabs/ConflictBar";
import { type Tab, neighbourId } from "./features/tabs/model";
import { TabStrip } from "./features/tabs/TabStrip";
import { useActiveTab, useTabs } from "./features/tabs/tabsStore";

type Overlay = "palette" | "shortcuts" | null;

const EDITOR_FONT_FAMILY = "ui-monospace, SFMono-Regular, Menlo, monospace";

const activeId = () => useTabs.getState().activeId;
const activeTab = () => useTabs.getState().tabs.find((t) => t.id === activeId());

function stepTab(step: 1 | -1): void {
  const { tabs, activate } = useTabs.getState();
  const next = neighbourId(
    tabs.map((t) => t.id),
    activeId(),
    step,
  );
  if (next) activate(next);
}

function duplicateActive(): void {
  const tab = activeTab();
  if (tab) useTabs.getState().openLoose({ ...tab.draft, name: `${tab.draft.name} copy` });
}

/** Shows the collections and focuses the filter once the sidebar has rendered them. */
function focusTreeFilter(): void {
  useSidebar.getState().show("collections");
  setTimeout(() => {
    document.getElementById(TREE_FILTER_ID)?.focus();
  }, 0);
}

function toggleResponse(): void {
  const layout = useLayout.getState();
  layout.placeResponse(layout.split === "beside" ? "below" : "beside");
}

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
      "next-tab": () => {
        stepTab(1);
      },
      "previous-tab": () => {
        stepTab(-1);
      },
      duplicate: duplicateActive,
      beautify: () => {
        const tab = activeTab();
        if (tab) beautifyBody(tab);
      },
      "filter-tree": focusTreeFilter,
      "toggle-response": toggleResponse,
    }),
    [setOverlay],
  );
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
    item("next-tab", "Next tab"),
    item("previous-tab", "Previous tab"),
    item("duplicate", "Duplicate the request"),
    item("beautify", "Beautify the JSON body"),
    item("filter-tree", "Filter the requests"),
    item("toggle-response", "Put the response beside or below"),
    item("shortcuts", "Keyboard shortcuts"),
    ...appItems(sendTab),
  ];
}

/** Reopens what was open at the last quit, and follows changes on disk. */
function useCollectionsLifecycle() {
  useEffect(() => {
    void (async () => {
      void useUpdates.getState().check("startup");
      const failed = await useCollections.getState().restore();
      if (failed.length > 0) toast.error(`Could not reopen: ${failed.join(", ")}`);
      await useTabs.getState().restore();
    })();
    const stop = onCollectionChanged((changed) => {
      void useCollections.getState().refresh(changed.root);
      if (changed.paths.some((p) => p.startsWith("environments/"))) {
        void useEnvironments.getState().load(changed.root);
      }
      if (changed.paths.some((p) => p.startsWith("openapi/"))) {
        void useSpecs
          .getState()
          .load(changed.root)
          .catch(() => undefined);
      }
      void useTabs.getState().changedOnDisk(changed.root, changed.paths);
    });
    const unsubscribe = useCollections.subscribe((s) => {
      void syncSpecs(Object.keys(s.views));
    });
    const leaveSpec = useTabs.subscribe((s, previous) => {
      if (s.activeId !== previous.activeId) useSpecs.getState().close();
    });
    return () => {
      leaveSpec();
      unsubscribe();
      void stop.then((unlisten) => {
        unlisten();
      });
    };
  }, []);
}

function RequestView({ tab }: { tab: Tab }) {
  const variables = useScopeVariables(tab);
  const split = useRef<HTMLDivElement>(null);
  const placement = useLayout((s) => s.split);
  const share = useLayout(currentShare);
  const beside = placement === "beside";
  const size = `${String(share * 100)}%`;
  return (
    <VariablesContext.Provider value={variables}>
      <div ref={split} className={`flex min-h-0 flex-1 ${beside ? "flex-row" : "flex-col"}`}>
        <section
          aria-label="Request"
          className="flex min-h-0 min-w-0 flex-col gap-3 p-4"
          style={beside ? { width: size } : { height: size }}
        >
          <ConflictBar tab={tab} />
          <ShowInSpec tab={tab} />
          <RequestBar tab={tab} />
          <UsedVariables tab={tab} />
          <ExamplesBar tab={tab} />
          <RequestSections tab={tab} />
        </section>
        <SplitResizer
          container={split}
          split={placement}
          share={share}
          onResize={useLayout.getState().resizeRequest}
        />
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          <PanelBoundary name="Response">
            <SchemaCheck tab={tab} />
            <ResponsePanel status={tab.status} previous={tab.previous} />
          </PanelBoundary>
        </div>
      </div>
    </VariablesContext.Provider>
  );
}

/** Collections or history, and the cURL import. */
function Sidebar() {
  const side = useSidebar((s) => s.side);
  const setSide = useSidebar.getState().show;
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
            useDialogs.getState().show("curl");
          }}
        >
          Import cURL…
        </button>
        <button
          type="button"
          className="text-xs text-zinc-400 hover:text-zinc-100"
          onClick={() => {
            useDialogs.getState().show("openapi");
          }}
        >
          OpenAPI…
        </button>
        <button
          type="button"
          className="text-xs text-zinc-400 hover:text-zinc-100"
          onClick={() => {
            useDialogs.getState().show("interop");
          }}
        >
          Postman…
        </button>
        <button
          type="button"
          aria-label="Settings"
          title="Settings"
          className="text-xs text-zinc-400 hover:text-zinc-100"
          onClick={() => {
            useDialogs.getState().show("settings");
          }}
        >
          ⚙
        </button>
      </div>
      <PanelBoundary name={side === "collections" ? "Collections" : "History"}>
        {side === "collections" ? <CollectionTree /> : <HistoryPanel />}
      </PanelBoundary>
      {side === "collections" && (
        <PanelBoundary name="API specs">
          <SpecList />
        </PanelBoundary>
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
  const themeSetting = useSettings((s) => s.theme);
  const fontSize = useSettings((s) => s.fontSize);
  const theme = resolveTheme(themeSetting, usePrefersDark());
  useApplyResolvedTheme(theme);
  const actions = useShortcutActions(setOverlay);
  const editorSettings = useMemo(
    () => ({ theme, font: { family: EDITOR_FONT_FAMILY, size: fontSize } }),
    [theme, fontSize],
  );
  const tab = useActiveTab();
  const spec = useSpecs((s) => s.view);
  const sidebarWidth = useLayout((s) => s.sidebarWidth);
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
        <aside
          className="flex shrink-0 flex-col bg-zinc-900"
          style={{ width: `${String(sidebarWidth)}px` }}
        >
          <Sidebar />
        </aside>
        <SidebarResizer
          width={sidebarWidth}
          min={SIDEBAR_WIDTH.min}
          max={SIDEBAR_WIDTH.max}
          onResize={useLayout.getState().resizeSidebar}
        />
        <main className="flex min-w-0 flex-1 flex-col">
          <TabStrip />
          {spec ? (
            <PanelBoundary name="Spec">
              <SpecBrowser view={spec} />
            </PanelBoundary>
          ) : tab ? (
            <RequestView key={tab.id} tab={tab} />
          ) : (
            <EmptyState />
          )}
        </main>
      </div>
      {overlay === "palette" && <CommandPalette commands={paletteItems(actions)} onClose={close} />}
      {overlay === "shortcuts" && (
        <ShortcutsView registry={registry} groups={SHORTCUT_GROUPS} onClose={close} />
      )}
      <SendConfirm />
      <AppDialogs />
      <UpdatePrompt />
      <RunnerDialog />
      <Toaster />
    </EditorSettingsProvider>
  );
}
