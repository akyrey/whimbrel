import { createUpdateStore } from "@roost/updates";

import { toAppError, updateCheck, updateInstall } from "../../ipc";

/** Updates from Whimbrel's GitHub releases (FR-9.2). */
export const useUpdates = createUpdateStore({
  appName: "Whimbrel",
  check: updateCheck,
  install: updateInstall,
  describeError: (caught) => toAppError(caught).message,
});
