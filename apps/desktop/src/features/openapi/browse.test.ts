import { describe, expect, it } from "vitest";

import type { Api, Operation } from "../../ipc";
import { refName, schemaChildren, schemaNotes, tagTree, typeLabel } from "./browse";

const op = (id: string, tags: string[], path = `/${id}`): Operation => ({
  id,
  hasOperationId: true,
  method: "GET",
  path,
  summary: null,
  description: null,
  tags,
  deprecated: false,
  parameters: [],
  requestBody: null,
  responses: [],
  security: null,
  callbacks: [],
});

const api: Api = {
  version: "open_api32",
  versionText: "3.2.0",
  title: "T",
  apiVersion: "1",
  description: null,
  servers: [],
  tags: [
    { name: "search", summary: "Search", description: null, parent: null, kind: "nav" },
    { name: "advanced", summary: null, description: null, parent: "search", kind: null },
    { name: "beta", summary: null, description: null, parent: null, kind: "badge" },
  ],
  operations: [op("a", ["search"]), op("b", ["beta", "advanced"]), op("c", []), op("d", ["extra"])],
  webhooks: [],
  securitySchemes: [],
  security: [],
  schemas: {},
  warnings: [],
};

describe("tag tree", () => {
  it("nests by parent, skips badge tags and puts untagged ones last", () => {
    const tree = tagTree(api);
    expect(tree.map((n) => n.label)).toEqual(["Search", "extra", "Other"]);
    expect(tree[0]?.children[0]?.operations.map((o) => o.id)).toEqual(["b"]);
    expect(tree[2]?.operations.map((o) => o.id)).toEqual(["c"]);
  });

  it("filters operations and drops empty branches", () => {
    const tree = tagTree(api, "get /b");
    expect(tree.map((n) => n.name)).toEqual(["search"]);
    expect(tree[0]?.operations).toEqual([]);
  });

  it("survives a parent cycle", () => {
    const cyclic: Api = {
      ...api,
      tags: [
        { name: "x", summary: null, description: null, parent: "y", kind: null },
        { name: "y", summary: null, description: null, parent: "x", kind: null },
      ],
      operations: [op("a", ["x"])],
    };
    expect(tagTree(cyclic)).toEqual([]);
  });
});

describe("schema labels", () => {
  it("names types, links and combinations", () => {
    expect(refName({ $ref: "#/schemas/Pet" })).toBe("Pet");
    expect(refName({ type: "string" })).toBeNull();
    expect(typeLabel({ type: "string", format: "uuid" })).toBe("string (uuid)");
    expect(typeLabel({ type: ["string", "null"] })).toBe("string | null");
    expect(typeLabel({ type: "array", items: { $ref: "#/schemas/Pet" } })).toBe("array of Pet");
    expect(typeLabel({ oneOf: [{}, {}] })).toBe("oneOf (2)");
    expect(typeLabel({ const: "x" })).toBe('const "x"');
    expect(typeLabel({ properties: {} })).toBe("object");
    expect(typeLabel(true)).toBe("any");
  });

  it("lists children and notes", () => {
    const schema = {
      type: "object",
      required: ["id"],
      properties: { id: { type: "integer" }, tags: { type: "array", items: { type: "string" } } },
      additionalProperties: { type: "string" },
    };
    expect(schemaChildren(schema).map((c) => [c.name, c.required])).toEqual([
      ["id", true],
      ["tags", false],
      ["{key}", false],
    ]);
    expect(schemaChildren({ prefixItems: [{}, {}] }).map((c) => c.name)).toEqual(["[0]", "[1]"]);
    expect(schemaNotes({ enum: ["a"], default: "a", minimum: 1, readOnly: true })).toEqual([
      'one of "a"',
      'default "a"',
      "minimum 1",
      "read-only",
    ]);
  });
});
