import { UpdatePrompt as KitUpdatePrompt } from "@roost/updates";

import { isDirty } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { useUpdates } from "./updateStore";

/** What a restart would put at risk: unsaved edits survive it, but say so. */
function unsavedWarning(): string | null {
  const dirty = useTabs.getState().tabs.filter((t) => t.file !== null && isDirty(t)).length;
  if (dirty === 0) return null;
  return `${String(dirty)} tab${dirty === 1 ? " has" : "s have"} unsaved edits; they are kept and come back after the restart.`;
}

/** Offers a found update, and shows the download until the app restarts. */
export function UpdatePrompt() {
  return (
    <KitUpdatePrompt appName="Whimbrel" useUpdates={useUpdates} unsavedWarning={unsavedWarning} />
  );
}
