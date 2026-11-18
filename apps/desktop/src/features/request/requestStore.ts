import { create } from "zustand";

import {
  type AppError,
  type Body,
  type BodyKind,
  type Header,
  type Request,
  type Response,
  requestCancel,
  requestSend,
  toAppError,
} from "../../ipc";

/** Phase 1 has one tab; Phase 2 gives each tab its id. */
export const TAB_ID = "main";

export type SendStatus =
  | { kind: "idle" }
  | { kind: "sending" }
  | { kind: "done"; response: Response }
  | { kind: "failed"; error: AppError };

export const EMPTY_REQUEST: Request = {
  method: "GET",
  url: "",
  headers: [],
  body: { type: "none" },
};

const EMPTY_HEADER: Header = { name: "", value: "", enabled: true };

type RequestState = {
  request: Request;
  status: SendStatus;
  setMethod: (method: string) => void;
  setUrl: (url: string) => void;
  addHeader: () => void;
  updateHeader: (index: number, change: Partial<Header>) => void;
  removeHeader: (index: number) => void;
  setBodyKind: (kind: BodyKind | "none") => void;
  setBodyText: (text: string) => void;
  send: () => Promise<void>;
  cancel: () => Promise<void>;
};

function withBodyKind(body: Body, kind: BodyKind | "none"): Body {
  if (kind === "none") return { type: "none" };
  return { type: "text", kind, text: body.type === "text" ? body.text : "" };
}

export const useRequest = create<RequestState>()((set, get) => {
  const update = (change: (request: Request) => Request) => {
    set({ request: change(get().request) });
  };
  return {
    request: EMPTY_REQUEST,
    status: { kind: "idle" },
    setMethod: (method) => {
      update((r) => ({ ...r, method: method.trim().toUpperCase() }));
    },
    setUrl: (url) => {
      update((r) => ({ ...r, url }));
    },
    addHeader: () => {
      update((r) => ({ ...r, headers: [...r.headers, EMPTY_HEADER] }));
    },
    updateHeader: (index, change) => {
      update((r) => ({
        ...r,
        headers: r.headers.map((h, i) => (i === index ? { ...h, ...change } : h)),
      }));
    },
    removeHeader: (index) => {
      update((r) => ({ ...r, headers: r.headers.filter((_, i) => i !== index) }));
    },
    setBodyKind: (kind) => {
      update((r) => ({ ...r, body: withBodyKind(r.body, kind) }));
    },
    setBodyText: (text) => {
      update((r) => (r.body.type === "text" ? { ...r, body: { ...r.body, text } } : r));
    },
    send: async () => {
      if (get().status.kind === "sending") return;
      set({ status: { kind: "sending" } });
      try {
        const response = await requestSend(TAB_ID, get().request);
        set({ status: { kind: "done", response } });
      } catch (caught) {
        set({ status: { kind: "failed", error: toAppError(caught) } });
      }
    },
    cancel: async () => {
      if (get().status.kind !== "sending") return;
      // The pending send rejects with `cancelled`, which ends the sending state.
      await requestCancel(TAB_ID).catch(() => false);
    },
  };
});
