import type { Collection } from "../../ipc";
import { inheritedFrom } from "./inheritance";

const collection: Collection = {
  id: "c",
  name: "Shop",
  children: [
    {
      kind: "folder",
      path: "admin",
      name: "Admin",
      order: 0,
      children: [{ kind: "folder", path: "admin/users", name: "Users", order: 0, children: [] }],
    },
  ],
};

describe("inheritedFrom", () => {
  it("lists the request's folders, innermost first, then the collection", () => {
    expect(inheritedFrom(collection, "admin/users/list.req.toml")).toEqual([
      { folder: "admin/users", name: "Users" },
      { folder: "admin", name: "Admin" },
      { folder: "", name: "Shop" },
    ]);
  });

  it("lists only the collection for a request at the top", () => {
    expect(inheritedFrom(collection, "ping.req.toml")).toEqual([{ folder: "", name: "Shop" }]);
  });

  it("names a folder missing from the tree by its directory", () => {
    expect(inheritedFrom(collection, "gone/x.req.toml")).toEqual([
      { folder: "gone", name: "gone" },
      { folder: "", name: "Shop" },
    ]);
  });
});
