/** Edits of the active tab's request, as functions of the draft. Pure. */
import type { Body, BodyKind, FormField, Header, StoredRequest } from "../../ipc";

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

/** What the body type menu offers. */
export type BodyChoice = BodyKind | "none" | "form" | "multipart" | "file";

export function choiceOf(body: Body): BodyChoice {
  return body.type === "text" ? body.kind : body.type;
}

const fieldsOf = (body: Body): FormField[] =>
  body.type === "form" || body.type === "multipart" ? body.fields : [];

/** The body as `choice`, keeping what carries over (text between text kinds, fields between
 * form and multipart; a form drops file fields). */
function withKind(body: Body, choice: BodyChoice): Body {
  switch (choice) {
    case "none":
      return { type: "none" };
    case "form":
      return { type: "form", fields: fieldsOf(body).filter((f) => !f.file) };
    case "multipart":
      return { type: "multipart", fields: fieldsOf(body) };
    case "file":
      return { type: "file", path: body.type === "file" ? body.path : "" };
    default:
      return { type: "text", kind: choice, text: body.type === "text" ? body.text : "" };
  }
}

export const setBodyKind = (choice: BodyChoice): Change =>
  request((r) => ({ ...r, body: withKind(r.body, choice) }));

const EMPTY_FIELD: FormField = { name: "", value: "", enabled: true, file: false };

const withFields = (change: (fields: FormField[]) => FormField[]): Change =>
  request((r) =>
    r.body.type === "form" || r.body.type === "multipart"
      ? { ...r, body: { ...r.body, fields: change(r.body.fields) } }
      : r,
  );

export const addField: Change = withFields((fields) => [...fields, EMPTY_FIELD]);

export const updateField = (index: number, change: Partial<FormField>): Change =>
  withFields((fields) => fields.map((f, i) => (i === index ? { ...f, ...change } : f)));

export const removeField = (index: number): Change =>
  withFields((fields) => fields.filter((_, i) => i !== index));

export const setFilePath = (path: string): Change =>
  request((r) => (r.body.type === "file" ? { ...r, body: { type: "file", path } } : r));

export const setBodyText = (text: string): Change =>
  request((r) => (r.body.type === "text" ? { ...r, body: { ...r.body, text } } : r));
