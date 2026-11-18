/** Edits of the active tab's request, as functions of the draft. Pure. */
import type { Body, BodyKind, Header, StoredRequest } from "../../ipc";

type Change = (draft: StoredRequest) => StoredRequest;

const request = (change: (r: StoredRequest["request"]) => StoredRequest["request"]): Change => {
  return (draft) => ({ ...draft, request: change(draft.request) });
};

export const setMethod = (method: string): Change =>
  request((r) => ({ ...r, method: method.trim().toUpperCase() }));

export const setUrl = (url: string): Change => request((r) => ({ ...r, url }));

const EMPTY_HEADER: Header = { name: "", value: "", enabled: true };

export const addHeader: Change = request((r) => ({ ...r, headers: [...r.headers, EMPTY_HEADER] }));

export const updateHeader = (index: number, change: Partial<Header>): Change =>
  request((r) => ({
    ...r,
    headers: r.headers.map((h, i) => (i === index ? { ...h, ...change } : h)),
  }));

export const removeHeader = (index: number): Change =>
  request((r) => ({ ...r, headers: r.headers.filter((_, i) => i !== index) }));

function withKind(body: Body, kind: BodyKind | "none"): Body {
  if (kind === "none") return { type: "none" };
  return { type: "text", kind, text: body.type === "text" ? body.text : "" };
}

export const setBodyKind = (kind: BodyKind | "none"): Change =>
  request((r) => ({ ...r, body: withKind(r.body, kind) }));

export const setBodyText = (text: string): Change =>
  request((r) => (r.body.type === "text" ? { ...r, body: { ...r.body, text } } : r));
