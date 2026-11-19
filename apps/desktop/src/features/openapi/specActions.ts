/** Importing a spec: the command, then the collection opened or refreshed. Failures become
 * toasts; nothing here throws. */
import { toast } from "@roost/ui";

import {
  type ImportTarget,
  type SpecImported,
  type SpecSource,
  chooseFolder,
  specImport,
  toAppError,
} from "../../ipc";
import { useCollections } from "../collections/collectionsStore";
import { useSpecs } from "./specsStore";

/** "New collection" or the root of an open one. */
export type TargetChoice =
  { kind: "new"; name: string } | { kind: "existing"; root: string; folderName: string };

async function targetOf(choice: TargetChoice): Promise<ImportTarget | null> {
  if (choice.kind === "existing") {
    return { kind: "existing", root: choice.root, folderName: choice.folderName };
  }
  const folder = await chooseFolder("Choose a folder for the new collection");
  return folder === null ? null : { kind: "new", folder, name: choice.name };
}

export async function importSpec(
  source: SpecSource,
  allowRemote: boolean,
  choice: TargetChoice,
): Promise<SpecImported | null> {
  try {
    const target = await targetOf(choice);
    if (target === null) return null;
    const done = await specImport({ source, allowRemote, target });
    if (target.kind === "new") await useCollections.getState().open(done.root);
    else await useCollections.getState().refresh(done.root);
    await useSpecs.getState().load(done.root);
    const count = Object.keys(done.installed.requests).length;
    toast.success(`Imported ${String(count)} request${count === 1 ? "" : "s"}`);
    for (const warning of done.installed.warnings.slice(0, 3)) toast.info(warning);
    return done;
  } catch (caught) {
    toast.error(`Could not import: ${toAppError(caught).message}`);
    return null;
  }
}
