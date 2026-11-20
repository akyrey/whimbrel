import type { ReadRequest } from "../../ipc";
import {
  NEW_REQUEST,
  changedOnDisk,
  edited,
  neighbourId,
  fileTab,
  isDirty,
  isUnder,
  moved,
  resolved,
  savedAs,
  scratchTab,
} from "./model";

const file = { root: "/c", path: "users/list.req.toml" };
const read = (url: string, hash: string): ReadRequest => ({
  stored: { ...NEW_REQUEST, name: "List", request: { ...NEW_REQUEST.request, url } },
  hash,
});

describe("tab model", () => {
  it("is dirty after an edit and clean after saving", () => {
    const tab = fileTab(file, read("http://a", "h1"));
    const changed = edited(tab, {
      ...tab.draft,
      request: { ...tab.draft.request, url: "http://b" },
    });

    expect(isDirty(tab)).toBe(false);
    expect(isDirty(changed)).toBe(true);
    expect(isDirty(savedAs(changed, "h2"))).toBe(false);
    expect(savedAs(changed, "h2").hash).toBe("h2");
    expect(isDirty(scratchTab("s1")), "a scratch request is never saved").toBe(true);
  });

  it("follows the disk when saved, and records a conflict when not", () => {
    const tab = fileTab(file, read("http://a", "h1"));
    const own = changedOnDisk(tab, read("http://a", "h1"));
    const outside = changedOnDisk(tab, read("http://z", "h9"));
    const unsaved = edited(tab, { ...tab.draft, name: "Mine" });
    const clash = changedOnDisk(unsaved, read("http://z", "h9"));

    expect(own).toEqual(tab);
    expect(outside.draft.request.url).toBe("http://z");
    expect(outside.hash).toBe("h9");
    expect(clash.draft.name).toBe("Mine");
    expect(clash.conflict).toEqual({ disk: read("http://z", "h9").stored, hash: "h9" });
  });

  it("resolves a conflict by keeping mine or taking theirs", () => {
    const tab = changedOnDisk(
      edited(fileTab(file, read("http://a", "h1")), { ...NEW_REQUEST, name: "Mine" }),
      read("http://z", "h9"),
    );

    const mine = resolved(tab, "mine");
    const theirs = resolved(tab, "theirs");

    expect(mine.conflict).toBeNull();
    expect(mine.draft.name).toBe("Mine");
    expect(mine.hash, "the next save overwrites the disk").toBe("h9");
    expect(theirs.draft.request.url).toBe("http://z");
    expect(isDirty(theirs)).toBe(false);
    expect(resolved(theirs, "mine")).toBe(theirs);
  });

  it("follows a rename and knows what is inside a folder", () => {
    const tab = fileTab(file, read("http://a", "h1"));
    const renamed = moved(tab, { root: "/c", path: "users/all.req.toml" }, "All users");

    expect(renamed.draft.name).toBe("All users");
    expect(renamed.id).toBe("/c\u0000users/all.req.toml");
    expect(isUnder(tab, "/c", "users")).toBe(true);
    expect(isUnder(tab, "/c", "user")).toBe(false);
    expect(isUnder(tab, "/other", "users")).toBe(false);
    expect(isUnder(scratchTab("s"), "/c", "users")).toBe(false);
  });
});

describe("neighbourId", () => {
  const ids = ["a", "b", "c"];
  it("steps forward and back, wrapping at the ends", () => {
    expect(neighbourId(ids, "a", 1)).toBe("b");
    expect(neighbourId(ids, "c", 1)).toBe("a");
    expect(neighbourId(ids, "a", -1)).toBe("c");
  });
  it("starts at the first tab without an active one, and gives null without tabs", () => {
    expect(neighbourId(ids, null, 1)).toBe("a");
    expect(neighbourId(ids, "gone", -1)).toBe("a");
    expect(neighbourId([], null, 1)).toBeNull();
  });
});
