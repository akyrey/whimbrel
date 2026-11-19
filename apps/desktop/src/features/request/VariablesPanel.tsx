import { Button } from "@roost/ui";

import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { useState } from "react";

import { type Variables, freshName, renamed, withValue, without } from "./variables";

type NameProps = { index: number; name: string; onRename: (to: string) => void };

/** Edited locally and renamed on blur or Enter, so a name can be cleared while retyping it. */
function NameInput({ index, name, onRename }: NameProps) {
  const [text, setText] = useState(name);
  const commit = () => {
    const to = text.trim();
    if (to === "" || to === name) setText(name);
    else onRename(to);
  };
  return (
    <input
      aria-label={`Variable ${String(index + 1)} name`}
      className="input w-40 font-mono"
      value={text}
      onChange={(e) => {
        setText(e.target.value);
      }}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") commit();
      }}
    />
  );
}

/** The request's own `[variables]`: after the picked example, before the environment. */
export function VariablesPanel({ tab }: { tab: Tab }) {
  const { variables } = tab.draft;
  const entries = Object.entries(variables);
  const change = (next: (v: Variables) => Variables) => {
    useTabs.getState().edit(tab.id, (d) => ({ ...d, variables: next(d.variables) }));
  };
  return (
    <div className="flex flex-col gap-1">
      {entries.map(([name, value], i) => (
        <div key={name} className="flex items-center gap-2">
          <NameInput
            index={i}
            name={name}
            onRename={(to) => {
              change((v) => renamed(v, name, to));
            }}
          />
          <input
            aria-label={`Variable ${String(i + 1)} value`}
            className="input min-w-0 flex-1 font-mono"
            value={value}
            onChange={(e) => {
              change((v) => withValue(v, name, e.target.value));
            }}
          />
          <Button
            aria-label={`Remove variable ${name}`}
            onClick={() => {
              change((v) => without(v, name));
            }}
          >
            ✕
          </Button>
        </div>
      ))}
      <div>
        <Button
          onClick={() => {
            change((v) => withValue(v, freshName(v), ""));
          }}
        >
          Add variable
        </Button>
      </div>
    </div>
  );
}
