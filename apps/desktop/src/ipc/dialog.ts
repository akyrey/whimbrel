import { open } from "@tauri-apps/plugin-dialog";

/** Asks for a folder; `null` when the user cancels. */
export async function chooseFolder(title: string): Promise<string | null> {
  const chosen = await open({ directory: true, multiple: false, title });
  return typeof chosen === "string" ? chosen : null;
}
