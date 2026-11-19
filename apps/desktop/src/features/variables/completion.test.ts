import { describe as group, expect, it } from "vitest";

import type { ScopeVariable } from "../../ipc";
import { accept, completionAt, describe, matching, namesIn } from "./completion";

const v = (name: string, extra: Partial<ScopeVariable> = {}): ScopeVariable => ({
  name,
  value: "x",
  source: "environment",
  secret: false,
  set: true,
  ...extra,
});

group("completion", () => {
  it("finds the name being typed after {{", () => {
    expect(completionAt("{{base", 6)).toEqual({ start: 0, prefix: "base" });
    expect(completionAt("a/{{", 4)).toEqual({ start: 2, prefix: "" });
    expect(completionAt("{{base}}/x", 10)).toBeNull();
    expect(completionAt("no braces", 3)).toBeNull();
  });

  it("matches by prefix first, then anywhere", () => {
    const names = matching([v("userId"), v("baseUrl"), v("authBase")], "base").map((x) => x.name);
    expect(names).toEqual(["baseUrl", "authBase"]);
  });

  it("inserts the name with its braces, eating closing ones already there", () => {
    const at = { start: 0, prefix: "ba" };
    expect(accept("{{ba/users", 4, at, "baseUrl")).toEqual({
      text: "{{baseUrl}}/users",
      caret: 11,
    });
    expect(accept("{{ba}}/users", 4, at, "baseUrl")).toEqual({
      text: "{{baseUrl}}/users",
      caret: 11,
    });
  });

  it("lists the names a text uses", () => {
    expect(namesIn("{{a}}/{{ b }}/{{a}}/{{$uuid}}")).toEqual(["a", "b", "$uuid"]);
  });

  it("describes values, secrets and unknown names", () => {
    const vars = [
      v("host"),
      v("key", { secret: true, value: null }),
      v("unset", { secret: true, set: false, value: null }),
      v("$uuid", { source: "dynamic", value: null }),
    ];
    expect(describe("host", vars)).toEqual({
      text: "host = x (from the environment)",
      known: true,
    });
    expect(describe("key", vars).text).toBe("key = •••••• (secret, from the environment)");
    expect(describe("unset", vars).known).toBe(false);
    expect(describe("$uuid", vars).text).toBe("$uuid: generated when sent");
    expect(describe("nope", vars)).toEqual({ text: "nope is not defined", known: false });
  });
});
