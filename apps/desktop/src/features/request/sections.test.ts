import type { StoredRequest } from "../../ipc";
import { NEW_REQUEST } from "../tabs/model";
import { defaultSection, sectionLabel } from "./sections";

const withRequest = (patch: Partial<StoredRequest["request"]>): StoredRequest => ({
  ...NEW_REQUEST,
  request: { ...NEW_REQUEST.request, ...patch },
});

describe("request sections", () => {
  it("opens on the body when there is one, else on the query parameters", () => {
    expect(defaultSection(NEW_REQUEST)).toBe("params");
    expect(defaultSection(withRequest({ body: { type: "text", kind: "json", text: "{}" } }))).toBe(
      "body",
    );
  });

  it("labels count entries and mark what is set", () => {
    const draft: StoredRequest = {
      ...withRequest({
        url: "http://x/?a=1&b=2",
        headers: [{ name: "Accept", value: "*/*", enabled: true }],
        body: { type: "text", kind: "json", text: "{}" },
      }),
      auth: { type: "bearer", token: "{{token}}" },
      variables: { id: "1" },
    };
    expect(sectionLabel("params", draft)).toBe("Params (2)");
    expect(sectionLabel("headers", draft)).toBe("Headers (1)");
    expect(sectionLabel("variables", draft)).toBe("Variables (1)");
    expect(sectionLabel("auth", draft)).toBe("Auth •");
    expect(sectionLabel("body", draft)).toBe("Body •");
    expect(sectionLabel("checks", draft)).toBe("Checks");
    expect(sectionLabel("settings", draft)).toBe("Settings");
    expect(sectionLabel("auth", NEW_REQUEST)).toBe("Auth");
    expect(sectionLabel("body", NEW_REQUEST)).toBe("Body");
  });
});
