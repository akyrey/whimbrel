import { describe, expect, it } from "vitest";
import { freshName, renamed, withValue, without } from "./variables";

describe("request variables", () => {
  it("renames in place, keeping the order and refusing a taken name", () => {
    const vars = { a: "1", b: "2", c: "3" };
    expect(Object.keys(renamed(vars, "b", "x"))).toEqual(["a", "x", "c"]);
    expect(renamed(vars, "b", "c")).toBe(vars);
    expect(renamed(vars, "b", "b")).toBe(vars);
  });

  it("sets and removes values without changing the original", () => {
    const vars = { a: "1" };
    expect(withValue(vars, "a", "2")).toEqual({ a: "2" });
    expect(without(withValue(vars, "b", "x"), "a")).toEqual({ b: "x" });
    expect(vars).toEqual({ a: "1" });
  });

  it("finds a free name", () => {
    expect(freshName({})).toBe("variable");
    expect(freshName({ variable: "", variable2: "" })).toBe("variable3");
  });
});
