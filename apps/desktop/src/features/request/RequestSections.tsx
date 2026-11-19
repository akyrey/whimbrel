import { PanelBoundary } from "@roost/ui";
import type { KeyboardEvent } from "react";

import { ChecksEditor } from "../checks/ChecksEditor";
import type { Tab } from "../tabs/model";
import { AuthPanel } from "./AuthPanel";
import { BodyEditor } from "./BodyEditor";
import { HeadersEditor } from "./HeadersEditor";
import { QueryEditor } from "./QueryEditor";
import { SettingsPanel } from "./SettingsPanel";
import { VariablesPanel } from "./VariablesPanel";
import { SECTIONS, type Section, defaultSection, sectionLabel, useSections } from "./sections";

const tabId = (section: Section) => `request-section-${section}`;
const panelId = "request-section-panel";

function SectionBody({ section, tab }: { section: Section; tab: Tab }) {
  switch (section) {
    case "params":
      return <QueryEditor tab={tab} />;
    case "auth":
      return <AuthPanel tab={tab} />;
    case "headers":
      return (
        <PanelBoundary name="Headers">
          <HeadersEditor tab={tab} />
        </PanelBoundary>
      );
    case "body":
      return (
        <PanelBoundary name="Body">
          <BodyEditor tab={tab} />
        </PanelBoundary>
      );
    case "variables":
      return <VariablesPanel tab={tab} />;
    case "checks":
      return <ChecksEditor tab={tab} />;
    case "settings":
      return <SettingsPanel tab={tab} />;
  }
}

/** Arrow keys, Home and End move between tabs, as in the ARIA tabs pattern. */
function neighbour(current: Section, key: string): Section | null {
  const index = SECTIONS.indexOf(current);
  const last = SECTIONS.length - 1;
  const to: Record<string, number> = {
    ArrowRight: index === last ? 0 : index + 1,
    ArrowLeft: index === 0 ? last : index - 1,
    Home: 0,
    End: last,
  };
  const target = to[key];
  return target === undefined ? null : (SECTIONS[target] ?? null);
}

/** The request's parts as tabs: params, auth, headers, body, variables, checks, settings. */
export function RequestSections({ tab }: { tab: Tab }) {
  const chosen = useSections((s) => s.chosen[tab.id]);
  const section = chosen ?? defaultSection(tab.draft);
  const choose = (next: Section) => {
    useSections.getState().choose(tab.id, next);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const next = neighbour(section, event.key);
    if (!next) return;
    event.preventDefault();
    choose(next);
    document.getElementById(tabId(next))?.focus();
  };
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <div
        role="tablist"
        aria-label="Request parts"
        className="flex shrink-0 flex-wrap gap-x-4 gap-y-1 border-b border-zinc-800 text-sm"
        onKeyDown={onKeyDown}
      >
        {SECTIONS.map((s) => (
          <button
            key={s}
            id={tabId(s)}
            type="button"
            role="tab"
            aria-selected={section === s}
            aria-controls={panelId}
            tabIndex={section === s ? 0 : -1}
            className={
              section === s
                ? "-mb-px border-b-2 border-sky-500 pb-1 text-zinc-100"
                : "pb-1 text-zinc-400 hover:text-zinc-200"
            }
            onClick={() => {
              choose(s);
            }}
          >
            {sectionLabel(s, tab.draft)}
          </button>
        ))}
      </div>
      <div
        id={panelId}
        role="tabpanel"
        aria-labelledby={tabId(section)}
        className="flex min-h-0 flex-1 flex-col overflow-auto text-sm"
      >
        <SectionBody section={section} tab={tab} />
      </div>
    </div>
  );
}
