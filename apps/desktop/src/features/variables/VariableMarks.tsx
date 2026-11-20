import { type RefObject, useEffect, useLayoutEffect, useRef, useState } from "react";

import { explain, tokensIn } from "./completion";
import { useVariables } from "./scope";
import { type Hovered, VariableTooltip } from "./VariableTooltip";

/** Gives `box` the input's font, padding and border width, so the marks line up with its text. */
function mirror(box: HTMLElement, input: HTMLInputElement): void {
  const s = getComputedStyle(input);
  box.style.font = s.font;
  box.style.letterSpacing = s.letterSpacing;
  box.style.padding = s.padding;
  box.style.border = `${s.borderWidth} solid transparent`;
}

/** The variable whose mark contains the point, if any. */
function hit(marks: HTMLElement[], x: number, y: number): Hovered | null {
  for (const mark of marks) {
    const rect = mark.getBoundingClientRect();
    const inside = x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom;
    const name = mark.dataset.variable;
    if (inside && name !== undefined) return { name, rect };
  }
  return null;
}

/** Highlights the `{{variables}}` in a single-line input and explains the one under the
 * pointer. Drawn over the input without taking its clicks; follows its horizontal scroll. */
export function VariableMarks({
  text,
  input,
}: {
  text: string;
  input: RefObject<HTMLInputElement | null>;
}) {
  const variables = useVariables();
  const box = useRef<HTMLSpanElement>(null);
  const layer = useRef<HTMLSpanElement>(null);
  const [hovered, setHovered] = useState<Hovered | null>(null);

  // Layout, not state: the overlay copies the input's box and follows its scroll directly.
  useLayoutEffect(() => {
    if (box.current && input.current) mirror(box.current, input.current);
  }, [input]);

  const follow = (): void => {
    if (layer.current && input.current) {
      layer.current.style.transform = `translateX(${String(-input.current.scrollLeft)}px)`;
    }
  };
  // A new value can move the input's scroll (typing at the end of a long URL).
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useLayoutEffect(follow, [text]);

  useEffect(() => {
    const el = input.current;
    if (!el) return;
    const point = (e: MouseEvent) => {
      follow();
      const marks = layer.current ? [...layer.current.querySelectorAll<HTMLElement>("mark")] : [];
      setHovered(hit(marks, e.clientX, e.clientY));
    };
    const leave = () => {
      setHovered(null);
    };
    const following = ["scroll", "keyup", "mouseup", "focus", "blur"] as const;
    following.forEach((type) => {
      el.addEventListener(type, follow);
    });
    el.addEventListener("mousemove", point);
    el.addEventListener("mouseleave", leave);
    el.addEventListener("input", leave);
    return () => {
      following.forEach((type) => {
        el.removeEventListener(type, follow);
      });
      el.removeEventListener("mousemove", point);
      el.removeEventListener("mouseleave", leave);
      el.removeEventListener("input", leave);
    };
    // `follow` reads only refs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [input]);

  const tokens = tokensIn(text);
  const parts = tokens.flatMap((t, i) => {
    const known = explain(t.name, variables).known;
    const before = text.slice(tokens[i - 1]?.end ?? 0, t.start);
    return [
      before,
      <mark
        key={t.start}
        data-variable={t.name}
        data-known={String(known)}
        className={`rounded-sm text-transparent ${known ? "bg-sky-500/25" : "bg-rose-500/35"}`}
      >
        {text.slice(t.start, t.end)}
      </mark>,
    ];
  });
  const rest = text.slice(tokens.at(-1)?.end ?? 0);

  return (
    <>
      <span
        ref={box}
        aria-hidden
        className="pointer-events-none absolute inset-0 flex items-center overflow-hidden whitespace-pre text-transparent"
      >
        <span ref={layer}>
          {parts}
          {rest}
        </span>
      </span>
      {hovered && <VariableTooltip {...hovered} />}
    </>
  );
}
