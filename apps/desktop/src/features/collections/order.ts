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
