import { ContextMenu, type ContextMenuItem } from "@roost/ui";
import { useRef, useState } from "react";

import { useDialogs } from "../shell/dialogsStore";
import { openCollection } from "./actions";

type Props = {
  /** Asks for a folder and a name for a new collection. */
  onNewCollection: () => void;
};

type Point = { x: number; y: number };

/** The menu's gap below its button, in pixels. */
const MENU_OFFSET = 4;

function menuItems(onNewCollection: () => void): ContextMenuItem[] {
  const show = useDialogs.getState().show;
  return [
    { label: "Open collection…", onSelect: () => void openCollection() },
    { label: "New collection…", onSelect: onNewCollection },
    {
      label: "Import cURL…",
      onSelect: () => {
        show("curl");
      },
    },
    {
      label: "Import OpenAPI…",
      onSelect: () => {
        show("openapi");
      },
    },
    {
      label: "Import Postman or .http…",
      onSelect: () => {
        show("interop");
      },
    },
  ];
}

/** One small button for the actions used now and then: opening, creating and importing. */
export function CollectionsMenu({ onNewCollection }: Props) {
  const button = useRef<HTMLButtonElement>(null);
  const [at, setAt] = useState<Point | null>(null);
  const toggle = () => {
    const rect = button.current?.getBoundingClientRect();
    if (at || !rect) {
      setAt(null);
      return;
    }
    setAt({ x: rect.left, y: rect.bottom + MENU_OFFSET });
  };

  return (
    <>
      <button
        ref={button}
        type="button"
        aria-label="Add or import"
        aria-haspopup="menu"
        aria-expanded={at !== null}
        title="Open, create or import"
        className="shrink-0 rounded px-1.5 text-base leading-none text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
        // Keeps the open menu's outside-click handler from closing it before the toggle runs.
        onMouseDown={(e) => {
          e.stopPropagation();
        }}
        onClick={toggle}
      >
        +
      </button>
      {at && (
        <ContextMenu
          label="Add or import"
          x={at.x}
          y={at.y}
          items={menuItems(onNewCollection)}
          onClose={() => {
            setAt(null);
          }}
        />
      )}
    </>
  );
}
