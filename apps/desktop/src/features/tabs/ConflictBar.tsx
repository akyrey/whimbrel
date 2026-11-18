import { Button } from "@roost/ui";

import type { Tab } from "./model";
import { useTabs } from "./tabsStore";

/** Shown when the tab's file changed on disk under unsaved edits, or was removed. */
export function ConflictBar({ tab }: { tab: Tab }) {
  if (tab.missing) {
    return (
      <div role="alert" className="rounded bg-amber-950 px-3 py-2 text-sm text-amber-200">
        The file of this request is gone from disk. Save puts it back.
      </div>
    );
  }
  if (!tab.conflict) return null;
  const { resolve } = useTabs.getState();
  return (
    <div
      role="alert"
      className="flex items-center gap-3 rounded bg-amber-950 px-3 py-2 text-sm text-amber-200"
    >
      <span className="flex-1">This request changed on disk while you were editing it.</span>
      <Button
        onClick={() => {
          resolve(tab.id, "mine");
        }}
      >
        Keep mine
      </Button>
      <Button
        onClick={() => {
          resolve(tab.id, "theirs");
        }}
      >
        Take theirs
      </Button>
    </div>
  );
}
