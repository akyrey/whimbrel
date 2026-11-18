import { toast } from "@roost/ui";

import { curlExport, curlImport, toAppError } from "../../ipc";
import { NEW_REQUEST, type Tab } from "../tabs/model";
import { sendArgs, useTabs } from "../tabs/tabsStore";

/** Copies the resolved request as cURL; secret values only with `reveal`. */
export async function copyAsCurl(tab: Tab, reveal: boolean): Promise<void> {
  try {
    const command = await curlExport(sendArgs(tab), reveal);
    await navigator.clipboard.writeText(command);
    toast.success(
      reveal ? "Copied as cURL, with secret values" : "Copied as cURL (secrets masked)",
    );
  } catch (caught) {
    toast.error(`Could not copy as cURL: ${toAppError(caught).message}`);
  }
}

/** Opens a pasted cURL command as a new request; returns whether it could. */
export async function importCurl(text: string): Promise<boolean> {
  try {
    const imported = await curlImport(text);
    useTabs.getState().openLoose({
      ...NEW_REQUEST,
      name: "Imported request",
      request: imported.request,
      settings: imported.settings,
    });
    if (imported.ignored.length > 0) toast.info(`Not imported: ${imported.ignored.join(", ")}`);
    return true;
  } catch (caught) {
    toast.error(toAppError(caught).message);
    return false;
  }
}
