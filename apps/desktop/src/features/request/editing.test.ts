import { NEW_REQUEST } from "../tabs/model";
import {
  addHeader,
  removeHeader,
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
