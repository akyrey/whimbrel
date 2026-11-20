import { useState } from "react";

import type { Tab } from "../tabs/model";
import { describe, namesIn } from "./completion";
import { useVariables } from "./scope";
import { type Hovered, VariableTooltip } from "./VariableTooltip";

/** The texts of a request that hold variables. */
function textsOf(tab: Tab): string[] {
  const r = tab.draft.request;
  const body =
    r.body.type === "text"
      ? [r.body.text]
      : r.body.type === "file"
        ? [r.body.path]
        : r.body.type === "none"
          ? []
          : r.body.fields.flatMap((f) => [f.name, f.value]);
  return [r.url, ...r.headers.flatMap((h) => [h.name, h.value]), ...body];
}

/** The variables the request uses; hovering one shows its value and where it comes from. */
export function UsedVariables({ tab }: { tab: Tab }) {
  const variables = useVariables();
  const [hovered, setHovered] = useState<Hovered | null>(null);
  const names = [...new Set(textsOf(tab).flatMap(namesIn))];
  if (names.length === 0) return null;
  return (
    <ul aria-label="Variables used" className="flex flex-wrap gap-1 text-xs">
      {names.map((name) => {
        const { text, known } = describe(name, variables);
        return (
          <li
            key={name}
            tabIndex={0}
            aria-label={text}
            onMouseEnter={(e) => {
              setHovered({ name, rect: e.currentTarget.getBoundingClientRect() });
            }}
            onFocus={(e) => {
              setHovered({ name, rect: e.currentTarget.getBoundingClientRect() });
            }}
            onMouseLeave={() => {
              setHovered(null);
            }}
            onBlur={() => {
              setHovered(null);
            }}
            className={`rounded px-1.5 py-0.5 font-mono ${known ? "bg-zinc-800 text-zinc-300" : "bg-rose-950 text-rose-300"}`}
          >
            {name}
          </li>
        );
      })}
      {hovered && <VariableTooltip {...hovered} />}
    </ul>
  );
}
