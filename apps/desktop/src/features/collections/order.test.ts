import { describe, expect, it } from "vitest";

import type { Node } from "../../ipc";
import { moves, planDrop } from "./order";

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

describe("planning a drop", () => {
  const folder = (path: string, order: number, children: Node[]): Node => ({
    kind: "folder",
    path,
    name: path,
    order,
    children,
  });
  const tree: Node[] = [
    request("a.req.toml", 10),
    folder("users", 20, [request("users/list.req.toml", 10), folder("users/admin", 20, [])]),
    request("c.req.toml", 30),
  ];

  it("reorders within a folder, writing only what changes", () => {
    expect(planDrop(tree, "c.req.toml", "a.req.toml", "before")).toEqual({
      into: "",
      move: false,
      writes: [
        { path: "c.req.toml", order: 10 },
        { path: "a.req.toml", order: 20 },
        { path: "users", order: 30 },
      ],
    });
    expect(planDrop(tree, "a.req.toml", "users", "after")).toEqual({
      into: "",
      move: false,
      writes: [
        { path: "users", order: 10 },
        { path: "a.req.toml", order: 20 },
      ],
    });
  });

  it("moves into another folder at the drop position", () => {
    expect(planDrop(tree, "c.req.toml", "users/list.req.toml", "after")).toEqual({
      into: "users",
      move: true,
      writes: [
        { path: "c.req.toml", order: 20 },
        { path: "users/admin", order: 30 },
      ],
    });
  });

  it("moves into a folder or the collection, last", () => {
    expect(planDrop(tree, "a.req.toml", "users/admin", "into")).toEqual({
      into: "users/admin",
      move: true,
      writes: [{ path: "a.req.toml", order: 10 }],
    });
    expect(planDrop(tree, "users/list.req.toml", "", "into")).toEqual({
      into: "",
      move: true,
      writes: [{ path: "users/list.req.toml", order: 40 }],
    });
  });

  it("does nothing for a drop that changes nothing or cannot be done", () => {
    expect(planDrop(tree, "a.req.toml", "a.req.toml", "before")).toBeNull();
    expect(planDrop(tree, "a.req.toml", "users", "before")).toBeNull();
    expect(planDrop(tree, "a.req.toml", "", "into")).toBeNull();
    expect(planDrop(tree, "users", "users/admin", "into")).toBeNull();
    expect(planDrop(tree, "users", "users/list.req.toml", "before")).toBeNull();
    expect(planDrop(tree, "a.req.toml", "c.req.toml", "into")).toBeNull();
    expect(planDrop(tree, "gone", "a.req.toml", "before")).toBeNull();
    expect(planDrop(tree, "a.req.toml", "gone.req.toml", "after")).toBeNull();
  });
});
