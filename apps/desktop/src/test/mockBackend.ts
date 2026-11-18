/**
 * An in-memory stand-in for the Rust backend, for Playwright and App tests. It follows the same
 * contract as the real commands (src-tauri/src/commands.rs):
 * - `/echo` answers 200 with JSON describing what was sent;
 * - `/slow` waits until cancelled;
 * - host `unreachable.test` fails with `connect`;
 * - a URL without http(s):// fails with `invalid_request` on the `url` field;
 * - anything else answers 404 with a text body.
 */
import { mockIPC } from "@tauri-apps/api/mocks";

import type { AppError, Request, RequestSendArgs, Response } from "../ipc/generated";

type Pending = { reject: (error: AppError) => void };

function response(status: number, reason: string, contentType: string, text: string): Response {
  return {
    status,
    reason,
    httpVersion: "HTTP/1.1",
    url: "",
    headers: [
      { name: "content-type", value: contentType },
      { name: "x-mock", value: "1" },
    ],
    body: { text, size: text.length, truncated: false, contentType },
    timing: { headersMs: 12, totalMs: 15 },
  };
}

function answer(request: Request): Response | AppError {
  if (!/^https?:\/\//i.test(request.url)) {
    return {
      code: "invalid_request",
      message: "The URL needs http:// or https://",
      detail: { field: "url" },
    };
  }
  if (request.url.includes("unreachable.test")) {
    return { code: "connect", message: "Could not connect (connection failed)" };
  }
  if (request.url.includes("/echo")) {
    const sent = {
      method: request.method,
      headers: request.headers.filter((h) => h.enabled).map((h) => [h.name, h.value]),
      body: request.body.type === "text" ? request.body.text : "",
    };
    return { ...response(200, "OK", "application/json", JSON.stringify(sent)), url: request.url };
  }
  return { ...response(404, "Not Found", "text/plain", "no such route"), url: request.url };
}

const isError = (value: Response | AppError): value is AppError => "code" in value;

export function installMockBackend(): void {
  const pending = new Map<string, Pending>();
  mockIPC((command, payload) => {
    const args = (payload as { args: RequestSendArgs }).args;
    if (command === "request_send") {
      if (args.request.url.includes("/slow")) {
        return new Promise((_, reject: (error: AppError) => void) => {
          pending.set(args.requestId, { reject });
        });
      }
      const result = answer(args.request);
      return isError(result) ? Promise.reject(result) : Promise.resolve(result);
    }
    if (command === "request_cancel") {
      const running = pending.get(args.requestId);
      pending.delete(args.requestId);
      running?.reject({ code: "cancelled", message: "The request was cancelled" });
      return Promise.resolve(running !== undefined);
    }
    return Promise.reject({ code: "internal", message: `No mock for ${command}` });
  });
}
