/** Moving a node among its siblings: the orders to write. Pure. */
import type { Node } from "../../ipc";

const STEP = 10;

type Placed = { path: string; order: number };

const placed = (node: Node): Placed =>
  node.kind === "broken" ? { path: node.path, order: 0 } : { path: node.path, order: node.order };

/** The writes that move `siblings[index]` by `delta` (−1 up, +1 down): siblings are renumbered
 * 10, 20, … and only those whose order changes are written. */
export function moves(siblings: Node[], index: number, delta: -1 | 1): Placed[] {
  const target = index + delta;
  if (target < 0 || target >= siblings.length) return [];
  const list = siblings.map(placed);
  const moved = list.splice(index, 1);
  list.splice(target, 0, ...moved);
  return list
    .map((p, i) => ({ path: p.path, order: (i + 1) * STEP, before: p.order }))
    .filter((p) => p.order !== p.before)
    .map(({ path, order }) => ({ path, order }));
}

export type DropPosition = "before" | "after" | "into";

/** What a drop does: `move` the dragged node into `into` when its parent changes, then write
 * `writes` (paths as before the move). */
export type DropPlan = { into: string; move: boolean; writes: Placed[] };

const parentOf = (path: string) => path.split("/").slice(0, -1).join("/");
const isWithin = (path: string, folder: string) => path === folder || path.startsWith(`${folder}/`);

function find(nodes: Node[], path: string): Node | null {
  for (const node of nodes) {
    if (node.path === path) return node;
    if (node.kind === "folder" && path.startsWith(`${node.path}/`))
      return find(node.children, path);
  }
  return null;
}

/** The children of `folder` (`""`: the collection), or `null` when it is not a folder. */
function childrenOf(tree: Node[], folder: string): Node[] | null {
  if (folder === "") return tree;
  const node = find(tree, folder);
  return node?.kind === "folder" ? node.children : null;
}

/** Where the dragged node lands: the folder and the index among the others there. */
function destination(
  tree: Node[],
  dragged: string,
  target: string,
  position: DropPosition,
): { into: string; others: Node[]; index: number } | null {
  const into = position === "into" ? target : parentOf(target);
  const siblings = childrenOf(tree, into);
  if (siblings === null) return null;
  const others = siblings.filter((n) => n.path !== dragged);
  if (position === "into") return { into, others, index: others.length };
  const at = others.findIndex((n) => n.path === target);
  if (at === -1) return null;
  return { into, others, index: position === "before" ? at : at + 1 };
}

/** Plans dropping `dragged` before or after `target`, or into the folder `target` (`""`: the
 * collection), task 11.2. Siblings are renumbered 10, 20, … and only changed orders written;
 * `null` when the drop changes nothing or would put a folder inside itself. */
export function planDrop(
  tree: Node[],
  dragged: string,
  target: string,
  position: DropPosition,
): DropPlan | null {
  const node = find(tree, dragged);
  if (node === null || dragged === target) return null;
  const spot = destination(tree, dragged, target, position);
  if (spot === null || isWithin(spot.into, dragged)) return null;
  const move = spot.into !== parentOf(dragged);
  if (position === "into" && !move) return null;
  const list = [...spot.others.slice(0, spot.index), node, ...spot.others.slice(spot.index)];
  const writes = list
    .map((n, i) => ({ ...placed(n), order: (i + 1) * STEP, before: placed(n).order }))
    .filter((p) => p.order !== p.before || (move && p.path === dragged))
    .map(({ path, order }) => ({ path, order }));
  return writes.length === 0 ? null : { into: spot.into, move, writes };
}
