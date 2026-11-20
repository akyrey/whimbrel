import { type InputHTMLAttributes, useId, useLayoutEffect, useRef, useState } from "react";

import { type Completion, accept, completionAt, matching } from "./completion";
import { useVariables } from "./scope";
import { VariableMarks } from "./VariableMarks";

const MAX_SHOWN = 8;

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange"> & {
  value: string;
  onValue: (value: string) => void;
};

/** A text input that completes `{{variables}}` from the request's scope. */
export function VariableInput({ value, onValue, onKeyDown, ...rest }: Props) {
  const variables = useVariables();
  const input = useRef<HTMLInputElement>(null);
  const [at, setAt] = useState<Completion | null>(null);
  /** Where the caret goes once an accepted completion is in the value. */
  const pendingCaret = useRef<number | null>(null);
  useLayoutEffect(() => {
    const caret = pendingCaret.current;
    if (caret === null || !input.current) return;
    pendingCaret.current = null;
    input.current.setSelectionRange(caret, caret);
  }, [value]);
  const [active, setActive] = useState(0);
  const options = at ? matching(variables, at.prefix).slice(0, MAX_SHOWN) : [];
  const open = at !== null && options.length > 0;
  const listId = useId();

  const update = (text: string, caret: number) => {
    setAt(completionAt(text, caret));
    setActive(0);
  };
  const choose = (name: string) => {
    if (!at || !input.current) return;
    const caret = input.current.selectionStart ?? value.length;
    const done = accept(value, caret, at, name);
    pendingCaret.current = done.caret;
    onValue(done.text);
    setAt(null);
  };

  return (
    <span className="relative flex min-w-0 flex-1">
      <input
        {...rest}
        ref={input}
        value={value}
        aria-autocomplete="list"
        aria-controls={open ? listId : undefined}
        onChange={(e) => {
          onValue(e.target.value);
          update(e.target.value, e.target.selectionStart ?? e.target.value.length);
        }}
        onBlur={() => {
          setAt(null);
        }}
        onKeyDown={(e) => {
          if (open) {
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              const step = e.key === "ArrowDown" ? 1 : options.length - 1;
              setActive((active + step) % options.length);
              return;
            }
            if (e.key === "Enter" || e.key === "Tab") {
              e.preventDefault();
              const picked = options[active];
              if (picked) choose(picked.name);
              return;
            }
            if (e.key === "Escape") {
              e.preventDefault();
              setAt(null);
              return;
            }
          }
          onKeyDown?.(e);
        }}
      />
      <VariableMarks text={value} input={input} />
      {open && (
        <ul
          id={listId}
          role="listbox"
          aria-label="Variables"
          className="absolute left-0 top-full z-20 mt-1 max-h-60 w-72 overflow-y-auto rounded border border-zinc-700 bg-zinc-900 py-1 text-sm shadow-lg"
        >
          {options.map((v, i) => (
            <li
              key={v.name}
              role="option"
              aria-selected={i === active}
              className={`flex cursor-pointer justify-between gap-2 px-2 ${i === active ? "bg-zinc-800" : ""}`}
              onMouseDown={(e) => {
                e.preventDefault();
                choose(v.name);
              }}
            >
              <span className="font-mono">{v.name}</span>
              <span className="truncate text-xs text-zinc-500">
                {v.secret ? "secret" : (v.value ?? v.source)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </span>
  );
}
