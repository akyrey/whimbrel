/** Every keyboard shortcut in Whimbrel; lookup, labels and matching come from @roost/shortcuts. */
import { createShortcutRegistry, type Shortcut as KitShortcut } from "@roost/shortcuts";

export type ShortcutGroup = "General" | "Tabs" | "Request";

export type ShortcutId =
  "palette" | "shortcuts" | "new-tab" | "close-tab" | "save" | "send" | "cancel" | "focus-url";

export type Shortcut = KitShortcut<ShortcutId, ShortcutGroup>;

export const SHORTCUTS: readonly Shortcut[] = [
  { id: "palette", group: "General", description: "Command palette", keys: ["Mod-k"] },
  { id: "shortcuts", group: "General", description: "Keyboard shortcuts", keys: ["Mod-Shift-/"] },
  { id: "new-tab", group: "Tabs", description: "New request tab", keys: ["Mod-t"] },
  { id: "close-tab", group: "Tabs", description: "Close the tab", keys: ["Mod-w"] },
  { id: "save", group: "Request", description: "Save the request", keys: ["Mod-s"] },
  { id: "send", group: "Request", description: "Send the request", keys: ["Mod-Enter"] },
  { id: "cancel", group: "Request", description: "Cancel the request", keys: ["Mod-."] },
  { id: "focus-url", group: "Request", description: "Go to the URL", keys: ["Mod-l"] },
];

export const SHORTCUT_GROUPS: readonly ShortcutGroup[] = ["General", "Tabs", "Request"];

/** All shortcuts are window-level: the body editor does not bind any of these keys. */
export const registry = createShortcutRegistry(
  SHORTCUTS,
  SHORTCUTS.map((s) => s.id),
);

export const { shortcutLabel, matchShortcut } = registry;
