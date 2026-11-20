/** Every keyboard shortcut in Whimbrel; lookup, labels and matching come from @roost/shortcuts. */
import { createShortcutRegistry, type Shortcut as KitShortcut } from "@roost/shortcuts";

export type ShortcutGroup = "General" | "Tabs" | "Request" | "Layout";

export type ShortcutId =
  | "palette"
  | "shortcuts"
  | "new-tab"
  | "close-tab"
  | "next-tab"
  | "previous-tab"
  | "duplicate"
  | "save"
  | "send"
  | "cancel"
  | "focus-url"
  | "beautify"
  | "filter-tree"
  | "toggle-response";

export type Shortcut = KitShortcut<ShortcutId, ShortcutGroup>;

export const SHORTCUTS: readonly Shortcut[] = [
  { id: "palette", group: "General", description: "Command palette", keys: ["Mod-k"] },
  { id: "shortcuts", group: "General", description: "Keyboard shortcuts", keys: ["Mod-Shift-/"] },
  { id: "new-tab", group: "Tabs", description: "New request tab", keys: ["Mod-t"] },
  { id: "close-tab", group: "Tabs", description: "Close the tab", keys: ["Mod-w"] },
  {
    id: "next-tab",
    group: "Tabs",
    description: "Next tab",
    keys: ["Ctrl-Tab", "Mod-Alt-ArrowRight"],
  },
  {
    id: "previous-tab",
    group: "Tabs",
    description: "Previous tab",
    keys: ["Ctrl-Shift-Tab", "Mod-Alt-ArrowLeft"],
  },
  { id: "duplicate", group: "Tabs", description: "Duplicate the request", keys: ["Mod-Shift-d"] },
  { id: "save", group: "Request", description: "Save the request", keys: ["Mod-s"] },
  { id: "send", group: "Request", description: "Send the request", keys: ["Mod-Enter"] },
  { id: "cancel", group: "Request", description: "Cancel the request", keys: ["Mod-."] },
  { id: "focus-url", group: "Request", description: "Go to the URL", keys: ["Mod-l"] },
  {
    id: "beautify",
    group: "Request",
    description: "Beautify the JSON body",
    keys: ["Mod-Shift-b"],
  },
  { id: "filter-tree", group: "Layout", description: "Filter the requests", keys: ["Mod-p"] },
  {
    id: "toggle-response",
    group: "Layout",
    description: "Put the response beside or below",
    keys: ["Mod-j"],
  },
];

export const SHORTCUT_GROUPS: readonly ShortcutGroup[] = ["General", "Tabs", "Request", "Layout"];

/** All shortcuts are window-level: the body editor does not bind any of these keys (CodeMirror's
 * basic setup takes Mod-d, Mod-[ and Mod-], so they are not used here). */
export const registry = createShortcutRegistry(
  SHORTCUTS,
  SHORTCUTS.map((s) => s.id),
);

export const { shortcutLabel, matchShortcut } = registry;
