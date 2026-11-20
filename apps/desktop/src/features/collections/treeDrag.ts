/** Drag and drop in the collection tree (task 11.3): HTML5 drag events, the drag state, and the
 * drop through `dropNode`. Within one collection only. */
import { type DragEvent, useState } from "react";

import type { Node } from "../../ipc";
import { dropNode } from "./actions";
import { type DropPosition, dropPosition, planDrop } from "./order";

type Spot = { root: string; path: string };

export type Hover = Spot & { position: DropPosition };

export type TreeDrag = {
  hover: Hover | null;
  start: (spot: Spot) => void;
  /** Whether `spot` at `position` takes the dragged node; remembers it as the hovered target. */
  over: (spot: Spot, position: DropPosition) => boolean;
  leave: (spot: Spot) => void;
  drop: (spot: Spot, position: DropPosition) => void;
  end: () => void;
};

/** The drag state of the tree; `trees` gives each collection's nodes by root. */
export function useTreeDrag(trees: (root: string) => Node[] | undefined): TreeDrag {
  const [dragging, setDragging] = useState<Spot | null>(null);
  const [hover, setHover] = useState<Hover | null>(null);
  const end = () => {
    setDragging(null);
    setHover(null);
  };
  const allowed = (spot: Spot, position: DropPosition) => {
    const tree = trees(spot.root);
    if (!dragging || dragging.root !== spot.root || !tree) return false;
    return planDrop(tree, dragging.path, spot.path, position) !== null;
  };
  return {
    hover,
    start: (spot) => {
      setDragging(spot);
    },
    over: (spot, position) => {
      const ok = allowed(spot, position);
      const same =
        hover?.root === spot.root && hover.path === spot.path && hover.position === position;
      if (ok && !same) setHover({ ...spot, position });
      if (!ok && hover !== null) setHover(null);
      return ok;
    },
    leave: (spot) => {
      if (hover?.root === spot.root && hover.path === spot.path) setHover(null);
    },
    drop: (spot, position) => {
      const tree = trees(spot.root);
      if (dragging && tree && allowed(spot, position)) {
        void dropNode(spot.root, tree, dragging.path, spot.path, position);
      }
      end();
    },
    end,
  };
}

type Target = "request" | "folder" | "collection";

const positionOver = (event: DragEvent<HTMLElement>, target: Target): DropPosition => {
  if (target === "collection") return "into";
  const rect = event.currentTarget.getBoundingClientRect();
  return dropPosition(target, event.clientY - rect.top, rect.height);
};

/** The drag and drop props of a tree row; the collection's name is a drop target only. */
export function dragProps(drag: TreeDrag, spot: Spot, target: Target) {
  const overHere = (event: DragEvent<HTMLElement>) => {
    if (!drag.over(spot, positionOver(event, target))) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  };
  return {
    draggable: target !== "collection",
    onDragStart: (event: DragEvent<HTMLElement>) => {
      // Some engines start a drag only with data set.
      event.dataTransfer.setData("text/plain", spot.path);
      event.dataTransfer.effectAllowed = "move";
      drag.start(spot);
    },
    onDragEnter: overHere,
    onDragOver: overHere,
    onDragLeave: () => {
      drag.leave(spot);
    },
    onDrop: (event: DragEvent<HTMLElement>) => {
      event.preventDefault();
      drag.drop(spot, positionOver(event, target));
    },
    onDragEnd: drag.end,
  };
}

const HOVER_CLASS: Record<DropPosition, string> = {
  before: "shadow-[inset_0_2px_0_0] shadow-sky-500",
  after: "shadow-[inset_0_-2px_0_0] shadow-sky-500",
  into: "bg-sky-950 ring-1 ring-inset ring-sky-500",
};

/** The highlight of the hovered drop target, if `spot` is it. */
export function hoverClass(drag: TreeDrag, spot: Spot): string {
  const { hover } = drag;
  return hover?.root === spot.root && hover.path === spot.path ? HOVER_CLASS[hover.position] : "";
}
