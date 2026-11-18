import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";

import type { Request, Response } from "../../ipc";
import { EMPTY_REQUEST, useRequest } from "./requestStore";

const response: Response = {
  status: 201,
  reason: "Created",
  httpVersion: "HTTP/1.1",
  url: "http://localhost/users",
  headers: [{ name: "content-type", value: "application/json" }],
  body: { text: '{"id":1}', size: 8, truncated: false, contentType: "application/json" },
  timing: { headersMs: 3, totalMs: 4 },
};

beforeEach(() => {
  useRequest.setState({ request: EMPTY_REQUEST, status: { kind: "idle" } });
});

afterEach(() => {
  clearMocks();
});

describe("editing", () => {
  it("edits method, URL and headers without mutating the previous request", () => {
    const before = useRequest.getState().request;
    const s = useRequest.getState();
    s.setMethod(" post ");
    s.setUrl("http://localhost/users");
    s.addHeader();
    s.addHeader();
    useRequest.getState().updateHeader(1, { name: "X-Api", value: "a" });
    useRequest.getState().removeHeader(0);

    const after = useRequest.getState().request;
    expect(after).toEqual({
      ...EMPTY_REQUEST,
      method: "POST",
      url: "http://localhost/users",
      headers: [{ name: "X-Api", value: "a", enabled: true }],
    });
    expect(before).toBe(EMPTY_REQUEST);
  });

  it("keeps the body text across kinds and drops it for none", () => {
    const s = useRequest.getState();
    s.setBodyText("ignored without a body");
    s.setBodyKind("json");
    useRequest.getState().setBodyText('{"a":1}');
    useRequest.getState().setBodyKind("xml");
    expect(useRequest.getState().request.body).toEqual({
      type: "text",
      kind: "xml",
      text: '{"a":1}',
    });
    useRequest.getState().setBodyKind("none");
    expect(useRequest.getState().request.body).toEqual({ type: "none" });
  });
});

describe("sending", () => {
  it("sends the request for the tab and keeps the response", async () => {
    let sent: Request | undefined;
    mockIPC((command, payload) => {
      if (command === "request_send") {
        sent = (payload as { args: { request: Request } }).args.request;
        return response;
      }
      return null;
    });
    useRequest.getState().setUrl("http://localhost/users");

    await useRequest.getState().send();

    expect(sent?.url).toBe("http://localhost/users");
    expect(useRequest.getState().status).toEqual({ kind: "done", response });
  });

  it("keeps the error of a failed send", async () => {
    mockIPC(() => Promise.reject({ code: "connect", message: "Could not connect" }));

    await useRequest.getState().send();

    expect(useRequest.getState().status).toEqual({
      kind: "failed",
      error: { code: "connect", message: "Could not connect" },
    });
  });

  it("cancels only while sending, and the send ends as cancelled", async () => {
    let reject: ((e: unknown) => void) | undefined;
    const calls: string[] = [];
    mockIPC((command) => {
      calls.push(command);
      if (command === "request_send") {
        return new Promise((_, r) => {
          reject = r;
        });
      }
      reject?.({ code: "cancelled", message: "The request was cancelled" });
      return true;
    });

    await useRequest.getState().cancel();
    const sending = useRequest.getState().send();
    void useRequest.getState().send();
    await useRequest.getState().cancel();
    await sending;

    expect(calls).toEqual(["request_send", "request_cancel"]);
    expect(useRequest.getState().status).toMatchObject({
      kind: "failed",
      error: { code: "cancelled" },
    });
  });
});
