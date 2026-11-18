/** Every keyboard shortcut in Whimbrel; lookup, labels and matching come from @roost/shortcuts. */
import { createShortcutRegistry, type Shortcut as KitShortcut } from "@roost/shortcuts";

export type ShortcutGroup = "General" | "Request";

export type ShortcutId = "palette" | "shortcuts" | "send" | "cancel" | "focus-url";

export type Shortcut = KitShortcut<ShortcutId, ShortcutGroup>;

export const SHORTCUTS: readonly Shortcut[] = [
  { id: "palette", group: "General", description: "Command palette", keys: ["Mod-k"] },
  { id: "shortcuts", group: "General", description: "Keyboard shortcuts", keys: ["Mod-Shift-/"] },
  { id: "send", group: "Request", description: "Send the request", keys: ["Mod-Enter"] },
  { id: "cancel", group: "Request", description: "Cancel the request", keys: ["Mod-."] },
  { id: "focus-url", group: "Request", description: "Go to the URL", keys: ["Mod-l"] },
];

export const SHORTCUT_GROUPS: readonly ShortcutGroup[] = ["General", "Request"];

/** All shortcuts are window-level: the body editor does not bind any of these keys. */
export const registry = createShortcutRegistry(
  SHORTCUTS,
  SHORTCUTS.map((s) => s.id),
);

export const { shortcutLabel, matchShortcut } = registry;
