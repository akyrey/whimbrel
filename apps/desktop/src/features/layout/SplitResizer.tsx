import type { KeyboardEvent, PointerEvent, RefObject } from "react";

import { REQUEST_SHARE, type Split, initialShare } from "./layoutStore";

const KEY_STEP = 0.05;

type Props = {
  /** The element whose size the share is of. */
  container: RefObject<HTMLElement | null>;
  split: Split;
  share: number;
  onResize: (share: number) => void;
};

/** The draggable edge between the request and the response; arrow keys move it, a double
 * click puts it back. */
export function SplitResizer({ container, split, share, onResize }: Props) {
  const beside = split === "beside";
  const startDrag = (event: PointerEvent<HTMLDivElement>) => {
    const box = container.current?.getBoundingClientRect();
    const size = (beside ? box?.width : box?.height) ?? 0;
    if (size <= 0) return;
    event.preventDefault();
    const start = beside ? event.clientX : event.clientY;
    const onMove = (e: globalThis.PointerEvent) => {
      onResize(share + ((beside ? e.clientX : e.clientY) - start) / size);
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };
  const keys: Record<string, number> = beside
    ? { ArrowLeft: -KEY_STEP, ArrowRight: KEY_STEP }
    : { ArrowUp: -KEY_STEP, ArrowDown: KEY_STEP };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = keys[event.key];
    if (step === undefined) return;
    event.preventDefault();
    onResize(share + step);
  };
  return (
    <div
      role="separator"
      // A divider between side-by-side panes is a vertical line.
      aria-orientation={beside ? "vertical" : "horizontal"}
      aria-label="Resize the request and the response"
      aria-valuemin={Math.round(REQUEST_SHARE.min * 100)}
      aria-valuemax={Math.round(REQUEST_SHARE.max * 100)}
      aria-valuenow={Math.round(share * 100)}
      tabIndex={0}
      className={`${beside ? "w-1 cursor-col-resize" : "h-1 cursor-row-resize"} shrink-0 bg-zinc-800 hover:bg-sky-700 focus:bg-sky-700 focus:outline-none`}
      onPointerDown={startDrag}
      onKeyDown={onKeyDown}
      onDoubleClick={() => {
        onResize(initialShare(split));
      }}
    />
  );
}
