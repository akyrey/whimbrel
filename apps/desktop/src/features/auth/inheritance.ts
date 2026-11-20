import type { Collection, Node } from "../../ipc";

/** A folder (`folder` empty for the collection) whose auth and headers a request inherits. */
export type InheritSource = { folder: string; name: string };

function folderName(nodes: Node[], path: string): string | null {
  for (const node of nodes) {
    if (node.kind !== "folder") continue;
    if (node.path === path) return node.name;
    if (path.startsWith(`${node.path}/`)) return folderName(node.children, path);
  }
  return null;
}

/** Where a request at `path` inherits its auth from: its folders, innermost first, then the
 * collection (FR-5: the nearest one that does not itself inherit applies). */
export function inheritedFrom(collection: Collection, path: string): InheritSource[] {
  const dirs = path.split("/").slice(0, -1);
  const folders = dirs.map((dir, i) => {
    const folder = dirs.slice(0, i + 1).join("/");
    return { folder, name: folderName(collection.children, folder) ?? dir };
  });
  return [...folders.reverse(), { folder: "", name: collection.name }];
}
