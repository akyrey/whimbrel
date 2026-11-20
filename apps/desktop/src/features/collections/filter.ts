import type { Node } from "../../ipc";

const contains = (text: string, query: string) => text.toLowerCase().includes(query);

function keep(node: Node, query: string): Node | null {
  switch (node.kind) {
    case "broken":
      return contains(node.path, query) ? node : null;
    case "request":
      return [node.name, node.method, node.path].some((text) => contains(text, query))
        ? { ...node, examples: [] }
        : null;
    case "folder": {
      if (contains(node.name, query)) return node;
      const children = filterNodes(node.children, query);
      return children.length > 0 ? { ...node, children } : null;
    }
  }
}

/** The part of the tree that matches `query` (task 9.2): requests by name, method or file path,
 * with the folders above them; a folder whose name matches keeps everything in it. Examples
 * are left out of matching requests. A blank query returns `nodes` itself. */
export function filterNodes(nodes: Node[], query: string): Node[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return nodes;
  return nodes.flatMap((node) => {
    const kept = keep(node, needle);
    return kept === null ? [] : [kept];
  });
}
