import { open, save } from "@tauri-apps/plugin-dialog";

/** Asks for a folder; `null` when the user cancels. */
export async function chooseFolder(title: string): Promise<string | null> {
  const chosen = await open({ directory: true, multiple: false, title });
  return typeof chosen === "string" ? chosen : null;
}

/** Asks where to save a file; `null` when the user cancels. */
export async function chooseSaveFile(title: string, defaultPath: string): Promise<string | null> {
  return save({ title, defaultPath });
}

/** Asks for a file to open; `null` when the user cancels. */
export async function chooseFile(
  title: string,
  filters: { name: string; extensions: string[] }[],
): Promise<string | null> {
  const chosen = await open({ directory: false, multiple: false, title, filters });
  return typeof chosen === "string" ? chosen : null;
}
