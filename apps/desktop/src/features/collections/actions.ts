/** What the tree's menus do: the command, then the tree and the tabs brought up to date.
 * Failures become toasts; nothing here throws. */
import { toast } from "@roost/ui";

import {
  chooseFolder,
  chooseSaveFile,
  httpExport,
  folderCreate,
  nodeDelete,
  requestCreate,
  requestDuplicate,
  requestRename,
  toAppError,
} from "../../ipc";
import { NEW_REQUEST } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { useCollections } from "./collectionsStore";

async function attempt(what: string, run: () => Promise<void>): Promise<void> {
  try {
    await run();
  } catch (caught) {
    toast.error(`Could not ${what}: ${toAppError(caught).message}`);
  }
}

const refresh = (root: string) => useCollections.getState().refresh(root);

export const openCollection = () =>
  attempt("open the collection", async () => {
    const root = await chooseFolder("Open a collection");
    if (root) await useCollections.getState().open(root);
  });

/** Asks for the folder; the caller asks for the name. */
export const chooseCollectionFolder = () => chooseFolder("Choose a folder for the collection");

export const createCollection = (root: string, name: string) =>
  attempt("create the collection", async () => {
    await useCollections.getState().create(root, name);
  });

export const closeCollection = (root: string) =>
  attempt("close the collection", async () => {
    useTabs.getState().removed(root, "");
    await useCollections.getState().close(root);
  });

export const newRequest = (root: string, folder: string, name: string) =>
  attempt("create the request", async () => {
    const path = await requestCreate({ root, folder, stored: { ...NEW_REQUEST, name } });
    await refresh(root);
    await useTabs.getState().openFile({ root, path });
  });

export const newFolder = (root: string, parent: string, name: string) =>
  attempt("create the folder", async () => {
    await folderCreate(root, parent, name);
    await refresh(root);
  });

export const renameRequest = (root: string, path: string, name: string) =>
  attempt("rename the request", async () => {
    const to = await requestRename(root, path, name);
    useTabs.getState().renamed(root, path, to, name);
    await refresh(root);
  });

export const duplicateRequest = (root: string, path: string) =>
  attempt("duplicate the request", async () => {
    const copy = await requestDuplicate(root, path);
    await refresh(root);
    await useTabs.getState().openFile({ root, path: copy });
  });

export const deleteNode = (root: string, path: string) =>
  attempt("delete it", async () => {
    await nodeDelete(root, path);
    useTabs.getState().removed(root, path);
    await refresh(root);
  });

/** Writes the folder (`""`: the collection) as a `.http` file the user picks. */
export const exportHttp = (root: string, folder: string, name: string) =>
  attempt("export the requests", async () => {
    const destination = await chooseSaveFile("Export as .http", `${name}.http`);
    if (!destination) return;
    const count = await httpExport({ root, folder, destination });
    toast.success(`Exported ${String(count)} request${count === 1 ? "" : "s"}`);
  });
