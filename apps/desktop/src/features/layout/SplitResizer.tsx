import type { KeyboardEvent, PointerEvent, RefObject } from "react";

import { REQUEST_SHARE } from "./layoutStore";

const KEY_STEP = 0.05;

type Props = {
  /** The element whose height the share is of. */
  container: RefObject<HTMLElement | null>;
  share: number;
  onResize: (share: number) => void;
};

/** The draggable edge between the request and the response; arrow keys move it, a double
 * click puts it back. */
export function SplitResizer({ container, share, onResize }: Props) {
  const startDrag = (event: PointerEvent<HTMLDivElement>) => {
    const height = container.current?.getBoundingClientRect().height ?? 0;
    if (height <= 0) return;
    event.preventDefault();
    const startY = event.clientY;
    const onMove = (e: globalThis.PointerEvent) => {
      onResize(share + (e.clientY - startY) / height);
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = { ArrowUp: -KEY_STEP, ArrowDown: KEY_STEP }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    onResize(share + step);
  };
  return (
    <div
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize the request and the response"
      aria-valuemin={Math.round(REQUEST_SHARE.min * 100)}
      aria-valuemax={Math.round(REQUEST_SHARE.max * 100)}
      aria-valuenow={Math.round(share * 100)}
      tabIndex={0}
      className="h-1 shrink-0 cursor-row-resize bg-zinc-800 hover:bg-sky-700 focus:bg-sky-700 focus:outline-none"
      onPointerDown={startDrag}
      onKeyDown={onKeyDown}
      onDoubleClick={() => {
        onResize(REQUEST_SHARE.initial);
      }}
    />
  );
}
