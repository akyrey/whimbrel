# Phase 11: Drag and drop in the tree

Goal: requests and folders move by dragging them, within a folder and between folders, as
well as with "Move up" / "Move down".

Design: [PRD.md](../PRD.md) FR-1; [FILE_FORMAT.md](../FILE_FORMAT.md) (order, folders).

## Tasks

- [x] 11.1 `wbl-format::move_node` moves a request or a folder (with everything in it) into
      another folder of the same collection, last among its new siblings, under a unique file
      name; a folder cannot move into itself or one of its folders. The `node_move` command
      replaces `request_move`.
- [x] 11.2 Drop planning (pure): dropping a node before or after a sibling, or into a folder or
      the collection, gives the move (when the parent changes) and the fewest order writes,
      renumbering 10, 20, … like "Move up". Dropping a node on itself or into its own folder
      tree does nothing.
- [ ] 11.3 The tree: rows are draggable; over a request the upper half drops before it and the
      lower half after it; over a folder the upper quarter drops before it and the rest into
      it; over the collection's name, into the collection. The target is highlighted. Open tabs
      follow the files they show. Only within one collection.

## Notes

- HTML5 drag and drop. Tauri's `dragDropEnabled` blocks it only on Windows, which is not a
  target (PRD non-goals).

## Acceptance

Each task's tests pass; `wbl-format` and the frontend keep 80% line coverage.
