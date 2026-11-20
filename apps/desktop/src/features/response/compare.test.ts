import { type DiffLine, diffBodies, headerChanges, withContext } from "./compare";

describe("diffBodies", () => {
  it("marks lines as kept, removed or added, in order", () => {
    expect(diffBodies("a\nb\nc", "a\nB\nc\nd")).toEqual([
      { kind: "same", text: "a" },
      { kind: "removed", text: "b" },
      { kind: "added", text: "B" },
      { kind: "same", text: "c" },
      { kind: "added", text: "d" },
    ]);
  });

  it("finds no change between equal bodies", () => {
    expect(diffBodies("x\ny", "x\ny").every((l) => l.kind === "same")).toBe(true);
  });
});

describe("withContext", () => {
  const same = (n: number): DiffLine[] =>
    Array.from({ length: n }, (_, i) => ({ kind: "same", text: `s${String(i)}` }));

  it("keeps a few unchanged lines around each change and folds the rest", () => {
    const lines: DiffLine[] = [...same(10), { kind: "added", text: "new" }, ...same(10)];
    const shown = withContext(lines, 2);
    expect(shown).toEqual([
      { kind: "skipped", count: 8 },
      { kind: "same", text: "s8" },
      { kind: "same", text: "s9" },
      { kind: "added", text: "new" },
      { kind: "same", text: "s0" },
      { kind: "same", text: "s1" },
      { kind: "skipped", count: 8 },
    ]);
  });

  it("folds everything when nothing changed", () => {
    expect(withContext(same(5), 2)).toEqual([{ kind: "skipped", count: 5 }]);
  });
});

describe("headerChanges", () => {
  it("lists added, removed and changed headers by name, ignoring case and unchanged ones", () => {
    const before = [
      { name: "Content-Type", value: "application/json" },
      { name: "x-old", value: "1" },
      { name: "x-same", value: "s" },
      { name: "set-cookie", value: "a=1" },
      { name: "set-cookie", value: "b=2" },
    ];
    const after = [
      { name: "content-type", value: "text/plain" },
      { name: "X-Same", value: "s" },
      { name: "x-new", value: "2" },
      { name: "Set-Cookie", value: "a=1" },
      { name: "Set-Cookie", value: "b=2" },
    ];
    expect(headerChanges(before, after)).toEqual([
      { name: "content-type", before: "application/json", after: "text/plain" },
      { name: "x-new", before: null, after: "2" },
      { name: "x-old", before: "1", after: null },
    ]);
  });
});
