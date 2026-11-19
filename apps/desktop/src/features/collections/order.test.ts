import { describe, expect, it } from "vitest";

import type { Node } from "../../ipc";
import { moves } from "./order";

const request = (path: string, order: number): Node => ({
  kind: "request",
  path,
  name: path,
  order,
  method: "GET",
  examples: [],
});

describe("moving among siblings", () => {
  const siblings: Node[] = [
    request("a", 10),
    { kind: "folder", path: "f", name: "F", order: 20, children: [] },
    request("c", 30),
  ];

  it("swaps with the neighbour and writes only what changes", () => {
    expect(moves(siblings, 2, -1)).toEqual([
      { path: "c", order: 20 },
      { path: "f", order: 30 },
    ]);
    expect(moves(siblings, 0, 1)).toEqual([
      { path: "f", order: 10 },
      { path: "a", order: 20 },
    ]);
  });

  it("renumbers equal orders", () => {
    expect(moves([request("x", 0), request("y", 0)], 1, -1)).toEqual([
      { path: "y", order: 10 },
      { path: "x", order: 20 },
    ]);
  });

  it("does nothing past the ends", () => {
    expect(moves(siblings, 0, -1)).toEqual([]);
    expect(moves(siblings, 2, 1)).toEqual([]);
  });
});
