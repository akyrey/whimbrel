import { describe, expect, it } from "vitest";

import { joinUrl, queryRows, splitUrl, withRows } from "./query";

describe("query rows", () => {
  it("reads the query string as written", () => {
    expect(queryRows("https://x.test/a?q={{term}}&page=2&flag#top")).toEqual([
      { name: "q", value: "{{term}}" },
      { name: "page", value: "2" },
      { name: "flag", value: "", bare: true },
    ]);
    expect(queryRows("https://x.test/a")).toEqual([]);
    expect(queryRows("https://x.test/a?")).toEqual([]);
    expect(queryRows("https://x.test/a?x=1=2")).toEqual([{ name: "x", value: "1=2" }]);
  });

  it("round-trips URLs", () => {
    for (const url of [
      "https://x.test/a?q=1&r=%20#h",
      "{{baseUrl}}/users?limit=10",
      "https://x.test/",
      "https://x.test/?a",
    ]) {
      expect(joinUrl(splitUrl(url))).toBe(url);
    }
  });

  it("edits rows into the URL, keeping the fragment", () => {
    const url = "https://x.test/a?q=1#h";
    expect(withRows(url, (rows) => [...rows, { name: "page", value: "2" }])).toBe(
      "https://x.test/a?q=1&page=2#h",
    );
    expect(withRows(url, () => [])).toBe("https://x.test/a#h");
    expect(withRows("https://x.test", (rows) => [...rows, { name: "", value: "" }])).toBe(
      "https://x.test?=",
    );
  });
});
