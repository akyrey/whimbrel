import type { Header } from "../../ipc";
import { addHeader, inheritedRows, removeHeader, updateHeader, withInherited } from "./inherit";

const on = (name: string, value: string): Header => ({ name, value, enabled: true });
const off = (name: string, value: string): Header => ({ name, value, enabled: false });
const users = { folder: "users", name: "Users" };
const api = { folder: "", name: "API" };

describe("withInherited", () => {
  it("puts the collection's headers first, then each folder's, then the request's", () => {
    const levels = [[on("X-Area", "users")], [on("X-Tenant", "t"), on("Accept", "*/*")]];

    expect(withInherited(levels, [on("accept", "application/json")])).toEqual([
      on("X-Tenant", "t"),
      on("X-Area", "users"),
      on("accept", "application/json"),
    ]);
  });

  it("lets a deeper header, even a disabled one, replace every inherited one of its name", () => {
    const levels = [[], [on("Cookie", "a"), on("cookie", "b"), on("X-Keep", "1")]];

    expect(withInherited(levels, [off("COOKIE", "")])).toEqual([
      on("X-Keep", "1"),
      off("COOKIE", ""),
    ]);
  });
});

describe("inheritedRows", () => {
  it("lists every inherited header with its source and whether a deeper one replaces it", () => {
    const levels = [
      { source: users, headers: [on("Accept", "text/csv")] },
      { source: api, headers: [on("X-Tenant", "t"), on("Accept", "*/*"), on("X-Debug", "1")] },
    ];

    expect(inheritedRows(levels, [off("x-debug", "")])).toEqual([
      { header: on("X-Tenant", "t"), source: api, overridden: false },
      { header: on("Accept", "*/*"), source: api, overridden: true },
      { header: on("X-Debug", "1"), source: api, overridden: true },
      { header: on("Accept", "text/csv"), source: users, overridden: false },
    ]);
  });

  it("is empty when nothing is inherited", () => {
    expect(inheritedRows([{ source: api, headers: [] }], [on("A", "1")])).toEqual([]);
  });
});

describe("list edits", () => {
  it("add, update and remove without changing the list given", () => {
    const list = [on("A", "1")];

    expect(addHeader(list)).toEqual([on("A", "1"), on("", "")]);
    expect(updateHeader(0, { enabled: false })(list)).toEqual([off("A", "1")]);
    expect(removeHeader(0)(list)).toEqual([]);
    expect(list).toEqual([on("A", "1")]);
  });
});
