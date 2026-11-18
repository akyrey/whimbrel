import { formatSize, showBody, statusTone } from "./format";

const body = (text: string | null, contentType: string | null, size = 10) => ({
  text,
  contentType,
  size,
  truncated: false,
});

describe("showBody", () => {
  it("pretty-prints JSON, including +json types", () => {
    expect(showBody(body('{"a":[1]}', "application/json; charset=utf-8"))).toEqual({
      kind: "code",
      language: "json",
      text: '{\n  "a": [\n    1\n  ]\n}',
    });
    expect(showBody(body("[]", "application/problem+json"))).toMatchObject({ language: "json" });
  });

  it("shows broken JSON as it came, XML as XML, the rest as text", () => {
    expect(showBody(body("{nope", "application/json"))).toEqual({
      kind: "code",
      language: "text",
      text: "{nope",
    });
    expect(showBody(body("<a/>", "application/atom+xml"))).toMatchObject({ language: "xml" });
    expect(showBody(body("hi", null))).toMatchObject({ language: "text", text: "hi" });
  });

  it("knows empty and binary bodies", () => {
    expect(showBody(body("", "text/plain"))).toEqual({ kind: "empty" });
    expect(showBody(body(null, "image/png", 4))).toEqual({ kind: "binary", size: 4 });
  });
});

describe("formatSize and statusTone", () => {
  it.each([
    [0, "0 B"],
    [1023, "1023 B"],
    [1536, "1.5 KB"],
    [10 * 1024 * 1024, "10.0 MB"],
    [3 * 1024 ** 4, "3072.0 GB"],
  ])("%d bytes is %s", (bytes, text) => {
    expect(formatSize(bytes)).toBe(text);
  });

  it.each([
    [200, "ok"],
    [304, "redirect"],
    [404, "client"],
    [503, "server"],
  ])("%d is %s", (status, tone) => {
    expect(statusTone(status)).toBe(tone);
  });
});
