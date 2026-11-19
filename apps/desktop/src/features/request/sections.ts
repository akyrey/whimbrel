/** Which part of a request its editor shows: one tab per part, remembered per open tab. */
import { create } from "zustand";

import type { StoredRequest } from "../../ipc";
import { queryRows } from "./query";

export const SECTIONS = [
  "params",
  "auth",
  "headers",
  "body",
  "variables",
  "checks",
  "settings",
] as const;
export type Section = (typeof SECTIONS)[number];

const NAMES: Record<Section, string> = {
  params: "Params",
  auth: "Auth",
  headers: "Headers",
  body: "Body",
  variables: "Variables",
  checks: "Checks",
  settings: "Settings",
};

/** What a request opens on: its body when it has one, else its query parameters. */
export function defaultSection(draft: StoredRequest): Section {
  return draft.request.body.type === "none" ? "params" : "body";
}

const count = (n: number) => (n > 0 ? ` (${String(n)})` : "");

/** The tab's label: its name, with how many entries it holds, or a mark when it is set. */
export function sectionLabel(section: Section, draft: StoredRequest): string {
  const name = NAMES[section];
  switch (section) {
    case "params":
      return name + count(queryRows(draft.request.url).length);
    case "headers":
      return name + count(draft.request.headers.length);
    case "variables":
      return name + count(Object.keys(draft.variables).length);
    case "checks":
      return name + count(draft.captures.length + draft.assertions.length);
    case "auth":
      return draft.auth.type === "inherit" ? name : `${name} •`;
    case "body":
      return draft.request.body.type === "none" ? name : `${name} •`;
    case "settings":
      return name;
  }
}

type SectionsState = {
  chosen: Readonly<Record<string, Section>>;
  choose: (tabId: string, section: Section) => void;
};

export const useSections = create<SectionsState>()((set) => ({
  chosen: {},
  choose: (tabId, section) => {
    set((s) => ({ chosen: { ...s.chosen, [tabId]: section } }));
  },
}));
