/** What a run of a folder contains, worked out from the tree. Pure. */
import type { ExampleNode, Node, RunStep } from "../../ipc";

export type RunnableRequest = {
  path: string;
  name: string;
  method: string;
  examples: ExampleNode[];
};

/** `""`: as it is; `*`: once per example; anything else: that example's id. */
export type ModeChoice = string;
export const AS_IS = "";
export const EACH = "*";

/** The requests in `folder` (`""`: the whole collection) and below it, in tree order. */
export function requestsUnder(nodes: Node[], folder: string): RunnableRequest[] {
  const out: RunnableRequest[] = [];
  const walk = (list: Node[], inside: boolean) => {
    for (const node of list) {
      if (node.kind === "request" && inside) {
        out.push({
          path: node.path,
          name: node.name,
          method: node.method,
          examples: node.examples,
        });
      } else if (node.kind === "folder") {
        walk(node.children, inside || node.path === folder);
      }
    }
  };
  walk(nodes, folder === "");
  return out;
}

export function stepsOf(
  requests: RunnableRequest[],
  modes: Readonly<Record<string, ModeChoice>>,
): RunStep[] {
  return requests.map((r) => {
    const mode = modes[r.path] ?? AS_IS;
    if (mode === AS_IS) return { path: r.path, mode: { kind: "base" } };
    if (mode === EACH) return { path: r.path, mode: { kind: "each_example" } };
    return { path: r.path, mode: { kind: "example", id: mode } };
  });
}
