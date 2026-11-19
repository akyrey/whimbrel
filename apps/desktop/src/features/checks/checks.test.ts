import { describe, expect, it } from "vitest";

import { NEW_REQUEST } from "../tabs/model";
import {
  addAssertion,
  addCapture,
  blankAssertion,
  expectedText,
  parseExpected,
  removeAssertion,
  removeCapture,
  setAssertion,
  summary,
  updateCapture,
} from "./checks";

describe("checks helpers", () => {
  it("edits captures and assertions without changing the draft", () => {
    const one = addAssertion(addCapture(NEW_REQUEST));
    const named = updateCapture(0, { var: "token" })(one);
    expect(named.captures[0]?.var).toBe("token");
    expect(updateCapture(5, { var: "x" })(named)).toBe(named);
    const timed = setAssertion(0, blankAssertion("time"))(named);
    expect(timed.assertions[0]).toEqual({ check: "time", belowMs: 1000 });
    expect(removeAssertion(0)(removeCapture(0)(timed))).toEqual(NEW_REQUEST);
    expect(NEW_REQUEST.captures).toEqual([]);
  });

  it("blank assertions of every kind", () => {
    expect(blankAssertion("header")).toEqual({ check: "header", name: "", equals: null });
    expect(blankAssertion("json")).toEqual({ check: "json", path: "$.", equals: null });
  });

  it("reads expected values as JSON when they parse", () => {
    expect(parseExpected("42")).toBe(42);
    expect(parseExpected('"x"')).toBe("x");
    expect(parseExpected("plain")).toBe("plain");
    expect(parseExpected(" ")).toBeNull();
    expect(expectedText([1])).toBe("[1]");
    expect(expectedText("x")).toBe("x");
    expect(expectedText(null)).toBe("");
  });

  it("summarises results", () => {
    expect(summary(null)).toBeNull();
    expect(summary({ assertions: [], captures: [] })).toBeNull();
    expect(
      summary({
        assertions: [
          { passed: true, message: "" },
          { passed: false, message: "" },
        ],
        captures: [{ var: "a", secret: false, value: "1", error: null, saved: false }],
      }),
    ).toEqual({ passed: 2, total: 3 });
  });
});
