import { createPortal } from "react-dom";

import { explain } from "./completion";
import { useVariables } from "./scope";

/** A variable being pointed at, and the box it is drawn in. */
export type Hovered = { name: string; rect: DOMRect };

const GAP_PX = 4;
const MARGIN_PX = 8;
const WIDTH_PX = 384;

/** The value of a variable (secrets masked) and where it comes from, under `rect`. */
export function VariableTooltip({ name, rect }: Hovered) {
  const { known, value, note } = explain(name, useVariables());
  const left = Math.max(MARGIN_PX, Math.min(rect.left, window.innerWidth - WIDTH_PX - MARGIN_PX));
  return createPortal(
    <div
      role="tooltip"
      style={{ left, top: rect.bottom + GAP_PX, maxWidth: WIDTH_PX }}
      className="pointer-events-none fixed z-50 flex flex-col gap-0.5 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-xs shadow-lg"
    >
      <span className={`font-mono font-semibold ${known ? "text-sky-300" : "text-rose-300"}`}>
        {name}
      </span>
      {value !== null && (
        <span className="break-all font-mono text-zinc-100">
          {value === "" ? <em className="text-zinc-500">empty</em> : value}
        </span>
      )}
      <span className="text-zinc-400">{note}</span>
    </div>,
    document.body,
  );
}
