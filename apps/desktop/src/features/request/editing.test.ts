import { NEW_REQUEST } from "../tabs/model";
import {
  addField,
  addHeader,
  choiceOf,
  removeField,
  removeHeader,
  setFilePath,
  updateField,
  setBodyKind,
  setBodyText,
  setMethod,
  setUrl,
  updateHeader,
} from "./editing";

describe("request edits", () => {
  it("edits method, URL and headers without mutating the draft", () => {
    const draft = NEW_REQUEST;
    const after = [
      setMethod(" post "),
      setUrl("http://x"),
      addHeader,
      addHeader,
      updateHeader(1, { name: "X-Api" }),
      removeHeader(0),
    ].reduce((d, change) => change(d), draft);

    expect(after.request).toEqual({
      method: "POST",
      url: "http://x",
      headers: [{ name: "X-Api", value: "", enabled: true }],
      body: { type: "none" },
    });
    expect(draft.request.headers).toEqual([]);
  });

  it("keeps the body text across kinds and drops it for none", () => {
    const text = [
      setBodyText("ignored"),
      setBodyKind("json"),
      setBodyText("{}"),
      setBodyKind("xml"),
    ].reduce((d, change) => change(d), NEW_REQUEST);
    expect(text.request.body).toEqual({ type: "text", kind: "xml", text: "{}" });
    expect(setBodyKind("none")(text).request.body).toEqual({ type: "none" });
  });
});

describe("form, multipart and file bodies", () => {
  const base = NEW_REQUEST;
  const bodyOf = (d: typeof base) => d.request.body;

  it("switches kinds keeping what carries over", () => {
    const multipart = setBodyKind("multipart")(base);
    const withFields = updateField(1, { file: true, value: "/a.png" })(
      addField(updateField(0, { name: "title", value: "Hi" })(addField(multipart))),
    );
    expect(bodyOf(withFields)).toEqual({
      type: "multipart",
      fields: [
        { name: "title", value: "Hi", enabled: true, file: false },
        { name: "", value: "/a.png", enabled: true, file: true },
      ],
    });
    expect(bodyOf(setBodyKind("form")(withFields))).toEqual({
      type: "form",
      fields: [{ name: "title", value: "Hi", enabled: true, file: false }],
    });
    expect(bodyOf(removeField(0)(withFields))).toMatchObject({ fields: [{ value: "/a.png" }] });
    expect(choiceOf(bodyOf(withFields))).toBe("multipart");
  });

  it("edits a file body and ignores edits for other kinds", () => {
    const file = setFilePath("/tmp/x.bin")(setBodyKind("file")(base));
    expect(bodyOf(file)).toEqual({ type: "file", path: "/tmp/x.bin" });
    expect(bodyOf(setBodyKind("file")(file))).toEqual({ type: "file", path: "/tmp/x.bin" });
    expect(addField(file)).toEqual(file);
    expect(setFilePath("/y")(base)).toEqual(base);
    expect(choiceOf(bodyOf(setBodyKind("json")(base)))).toBe("json");
  });
});
