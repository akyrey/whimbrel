# Phase 2: Collections on disk

Goal: requests live in a collection folder as TOML files the user can commit, diff and merge;
the app opens, edits, saves, creates, renames, moves, duplicates and deletes them, and follows
changes made outside it (git checkout, pull, another editor).

Design: [FILE_FORMAT.md](../FILE_FORMAT.md), [PRD.md](../PRD.md) FR-1, FR-2.7, D-H03, D-H07.

## Tasks

- [x] 2.1 `wbl-format`: request, folder and collection files with `toml_edit`. Reading keeps the
      document; writing changes only the values that differ, keeping comments, the user's
      formatting and unknown keys. A file with `format` newer than the app's is refused.
- [x] 2.2 Property tests: write then read gives the same request; saving an unchanged request is
      byte-identical; editing one field changes only its line.
- [x] 2.3 Collection tree: `collection.toml` marks the root, folders are directories
      (`_folder.toml` optional), requests are `*.req.toml`, ordered by `order` then name.
      Unparsable files are listed as broken with the parser's message and never overwritten.
- [x] 2.4 Operations: create (file name from a slug of the name, made unique), rename (renames
      the file), move to a folder, duplicate, delete, new folder, reorder (`order` in steps of
      10, so a move rewrites one file). Writes go to a temporary file then rename.
- [x] 2.5 Commands: open a collection (folder dialog), read and save a request with the hash of
      what was read (`conflict` when the file changed since), and the operations above.
- [x] 2.6 File watcher (`notify`, debounced): a `collection_changed` event with the changed
      paths (hidden files and the writer's temporary files left out). The app's own saves are
      reported too and recognised in the frontend: the tab already holds the file's new hash.
- [x] 2.7 Frontend: the collection tree in the sidebar, tabs for open requests with an unsaved
      marker, save (Mod-s), a keep-mine / take-theirs prompt when an unsaved tab's file changes
      on disk, context menus for the operations, the open collections and tabs restored on start.
- [x] 2.8 E2E against the mocked backend: open a collection, edit and save, create, rename,
      delete, and an external change while a tab is unsaved.

## Notes

- Renaming and reordering folders, and drag-and-drop moves, have commands but no UI yet; the
  tree's menus cover new, rename, duplicate and delete. Picked up with Phase 8 polish.
- The `git diff` and `git checkout` acceptance checks need the real app on a real folder:
  covered by `wbl-format`'s byte-level tests and the watcher test, not yet by hand.

## Acceptance

- Editing a request in the app and saving changes only the edited lines (`git diff`).
- `git checkout` of another branch updates the tree and the open, saved tabs live.
- A broken file is shown as broken, and nothing the app does rewrites it.
- All gates as in Phase 1.
