import type { ResponseBody } from "../../ipc";
import {
  PRETTY_LIMIT,
  cookiesOf,
  formatSize,
  hexDump,
  imageSource,
  jsonTable,
  showBody,
  statusTone,
} from "./format";

const body = (
  text: string | null,
  contentType: string | null,
  size = 10,
  base64: string | null = null,
): ResponseBody => ({
  text,
  base64,
  contentType,
  size,
  truncated: false,
  file: null,
});

describe("showBody", () => {
  it("pretty-prints JSON, including +json types", () => {
    expect(showBody(body('{"a":[1]}', "application/json; charset=utf-8"))).toEqual({
      kind: "code",
      language: "json",
      text: '{\n  "a": [\n    1\n  ]\n}',
      pretty: true,
    });
    expect(showBody(body("[]", "application/problem+json"))).toMatchObject({ language: "json" });
  });

  it("shows broken JSON as it came, XML as XML, the rest as text", () => {
    expect(showBody(body("{nope", "application/json"))).toEqual({
      kind: "code",
      language: "text",
      text: "{nope",
      pretty: false,
    });
    expect(showBody(body("<a/>", "application/atom+xml"))).toMatchObject({ language: "xml" });
    expect(showBody(body("hi", null))).toMatchObject({ language: "text", text: "hi" });
  });

  it("leaves very long JSON as it came, so the window stays responsive", () => {
    const long = `[${"1,".repeat(PRETTY_LIMIT / 2)}1]`;
    expect(showBody(body(long, "application/json", long.length))).toMatchObject({
      language: "json",
      pretty: false,
      text: long,
    });
  });

  it("knows empty and binary bodies", () => {
    expect(showBody(body("", "text/plain"))).toEqual({ kind: "empty" });
    expect(showBody(body(null, "image/png", 4, "iVD//g=="))).toEqual({
      kind: "binary",
      size: 4,
      base64: "iVD//g==",
    });
  });
});

describe("binary", () => {
  it("dumps bytes as hex with their printable characters", () => {
    expect(hexDump(btoa("Hello, binary\u0000ÿ world!"))).toBe(
      "00000000  48 65 6c 6c 6f 2c 20 62  69 6e 61 72 79 00 ff 20  |Hello, binary.. |\n" +
        "00000010  77 6f 72 6c 64 21                                 |world!|",
    );
  });

  it("previews images, never other types", () => {
    expect(imageSource(body(null, "image/png", 4, "iVD//g=="))).toBe(
      "data:image/png;base64,iVD//g==",
    );
    expect(imageSource(body("<svg/>", "image/svg+xml"))).toBe(
      `data:image/svg+xml;base64,${btoa("<svg/>")}`,
    );
    expect(imageSource(body(null, "application/octet-stream", 4, "AAAA"))).toBeNull();
    expect(imageSource({ ...body(null, "image/png", 4, "iVD//g=="), truncated: true })).toBeNull();
  });
});

describe("jsonTable", () => {
  it("turns an array of objects into columns and rows", () => {
    expect(jsonTable('[{"id":1,"name":"ada"},{"id":2,"admin":true,"tags":["x"]}]')).toEqual({
      columns: ["id", "name", "admin", "tags"],
      rows: [
        [1, "ada", undefined, undefined],
        [2, undefined, true, ["x"]],
      ],
    });
  });

  it("is null for anything else", () => {
    for (const text of ["{}", "[]", "[1,2]", "not json", '[{"a":1}, 2]']) {
      expect(jsonTable(text)).toBeNull();
    }
  });
});

describe("cookies, sizes and statuses", () => {
  it("reads Set-Cookie headers", () => {
    expect(
      cookiesOf([
        { name: "set-cookie", value: "session=abc; Path=/; HttpOnly" },
        { name: "content-type", value: "text/plain" },
        { name: "Set-Cookie", value: "theme=dark" },
      ]),
    ).toEqual([
      { name: "session", value: "abc", attributes: "Path=/; HttpOnly" },
      { name: "theme", value: "dark", attributes: "" },
    ]);
  });

  it.each([
    [0, "0 B"],
    [1536, "1.5 KB"],
    [10 * 1024 * 1024, "10.0 MB"],
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
