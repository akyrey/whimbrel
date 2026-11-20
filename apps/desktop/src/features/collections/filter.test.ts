import type { Node } from "../../ipc";
import { filterNodes } from "./filter";

const request = (path: string, name: string, method = "GET"): Node => ({
  kind: "request",
  path,
  name,
  order: 0,
  method,
  examples: [{ id: "e", name: "Example" }],
});

const TREE: Node[] = [
  request("ping.req.toml", "Ping"),
  {
    kind: "folder",
    path: "users",
    name: "Users",
    order: 0,
    children: [
      request("users/list.req.toml", "List users"),
      request("users/new.req.toml", "Create", "POST"),
    ],
  },
  {
    kind: "folder",
    path: "session",
    name: "Session",
    order: 0,
    children: [request("session/login.req.toml", "Get a token", "POST")],
  },
  { kind: "broken", path: "bad.req.toml", error: "unparsable" },
];

const names = (nodes: Node[]): string[] =>
  nodes.flatMap((n) =>
    n.kind === "folder"
      ? [n.name, ...names(n.children).map((c) => `${n.name}/${c}`)]
      : [n.kind === "request" ? n.name : n.path],
  );

it("returns the tree unchanged for an empty or blank query", () => {
  expect(filterNodes(TREE, "")).toBe(TREE);
  expect(filterNodes(TREE, "   ")).toBe(TREE);
});

it("keeps requests whose name matches, case-insensitively, with their folders", () => {
  expect(names(filterNodes(TREE, "LIST"))).toEqual(["Users", "Users/List users"]);
});

it("matches the method and the file path", () => {
  expect(names(filterNodes(TREE, "post"))).toEqual([
    "Users",
    "Users/Create",
    "Session",
    "Session/Get a token",
  ]);
  expect(names(filterNodes(TREE, "login"))).toEqual(["Session", "Session/Get a token"]);
});

it("keeps all of a folder whose name matches", () => {
  expect(names(filterNodes(TREE, "users"))).toEqual(["Users", "Users/List users", "Users/Create"]);
});

it("keeps broken files whose path matches", () => {
  expect(names(filterNodes(TREE, "bad"))).toEqual(["bad.req.toml"]);
});

it("hides examples while filtering and leaves the input untouched", () => {
  const [ping] = filterNodes(TREE, "ping");
  expect(ping?.kind === "request" && ping.examples).toEqual([]);
  expect(TREE[0]?.kind === "request" && TREE[0].examples).toHaveLength(1);
});

it("returns nothing when nothing matches", () => {
  expect(filterNodes(TREE, "nope")).toEqual([]);
});
