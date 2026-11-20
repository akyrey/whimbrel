import { toast } from "@roost/ui";

import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { beautifyJson } from "./beautify";
import { setBodyText } from "./editing";

/** Pretty-prints the tab's JSON body as one edit that undo takes back; a toast says why not. */
export function beautifyBody(tab: Tab): void {
  const { body } = tab.draft.request;
  if (body.type !== "text" || body.kind !== "json") {
    toast.info("Only a JSON body can be beautified.");
    return;
  }
  const result = beautifyJson(body.text);
  if (!result.ok) {
    toast.error(`Not valid JSON: ${result.message}`);
    return;
  }
  if (result.text === body.text) return;
  // The editor follows the draft's text, so this is one edit that undo takes back.
  useTabs.getState().edit(tab.id, setBodyText(result.text));
}
