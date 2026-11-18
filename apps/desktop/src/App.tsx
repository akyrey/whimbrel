import { CommandPalette, type PaletteItem } from "@roost/palette";
import { ShortcutsView } from "@roost/shortcuts";
import {
  EditorSettingsProvider,
  PanelBoundary,
  Toaster,
  resolveTheme,
  useApplyResolvedTheme,
  usePrefersDark,
} from "@roost/ui";
import { useEffect, useMemo, useState } from "react";

import { BodyEditor } from "./features/request/BodyEditor";
import { HeadersEditor } from "./features/request/HeadersEditor";
import { RequestBar, URL_INPUT_ID } from "./features/request/RequestBar";
import { useRequest } from "./features/request/requestStore";
import { ResponsePanel } from "./features/response/ResponsePanel";
import {
  SHORTCUT_GROUPS,
  type ShortcutId,
  matchShortcut,
  registry,
  shortcutLabel,
} from "./features/shortcuts/registry";

type Overlay = "palette" | "shortcuts" | null;

const EDITOR_FONT = { family: "ui-monospace, SFMono-Regular, Menlo, monospace", size: 13 };

function focusUrl() {
  document.getElementById(URL_INPUT_ID)?.focus();
}

/** Runs a shortcut; returns whether it did anything. */
function useShortcutActions(setOverlay: (overlay: Overlay) => void) {
  return useMemo(
    (): Record<ShortcutId, () => void> => ({
      palette: () => {
        setOverlay("palette");
      },
      shortcuts: () => {
        setOverlay("shortcuts");
      },
      send: () => void useRequest.getState().send(),
      cancel: () => void useRequest.getState().cancel(),
      "focus-url": focusUrl,
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
    item("cancel", "Cancel the request"),
    item("focus-url", "Go to the URL"),
    item("shortcuts", "Keyboard shortcuts"),
  ];
}

export function App() {
  const [overlay, setOverlay] = useState<Overlay>(null);
  const theme = resolveTheme("system", usePrefersDark());
  useApplyResolvedTheme(theme);
  const actions = useShortcutActions(setOverlay);
  const editorSettings = useMemo(() => ({ theme, font: EDITOR_FONT }), [theme]);

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
      <main className="flex h-full flex-col">
        <section aria-label="Request" className="flex flex-col gap-3 border-b border-zinc-800 p-4">
          <RequestBar />
          <PanelBoundary name="Headers">
            <HeadersEditor />
          </PanelBoundary>
          <div className="flex h-56 flex-col">
            <PanelBoundary name="Body">
              <BodyEditor />
            </PanelBoundary>
          </div>
        </section>
        <PanelBoundary name="Response">
          <ResponsePanel />
        </PanelBoundary>
      </main>
      {overlay === "palette" && <CommandPalette commands={paletteItems(actions)} onClose={close} />}
      {overlay === "shortcuts" && (
        <ShortcutsView registry={registry} groups={SHORTCUT_GROUPS} onClose={close} />
      )}
      <Toaster />
    </EditorSettingsProvider>
  );
}
