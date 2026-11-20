# Phase 8: Polish, interop, release

Goal: the request editor covers FR-2 completely, collections move in and out of other tools,
the shell matches Dunlin's, and Whimbrel ships as signed, self-updating packages.

Design: [PRD.md](../PRD.md) FR-2, FR-9; [FILE_FORMAT.md](../FILE_FORMAT.md); D-H15, D-H18,
D-H19.

## Tasks

- [x] 8.1 Body kinds (FR-2.2): form-urlencoded and multipart (fields and files) and a binary
      file, in the file format, the engine and the editor; imported specs generate them.
- [x] 8.2 Query parameters (FR-2.3): a table that stays in sync with the URL both ways.
- [x] 8.3 Variables in the editor (FR-2.4): `{{` completes the names in scope (example, request,
      runtime, environment, collection, secrets by name), and hovering a variable shows its
      value (secrets masked) and where it comes from.
- [x] 8.4 Interop: import Postman v2.1 collections (secrets as secret names) and `.http` files;
      export a folder as `.http`.
- [x] 8.5 The tree: rename folders; move requests and folders up and down.
- [x] 8.6 The shell (FR-9.1, 9.4): palette commands for every action, settings (theme, editor
      font size, default timeout), and the open tabs restored.
- [x] 8.7 Spec browser leftovers from Phase 6: links and callbacks shown, `$self` as the base of
      references.
- [x] 8.8 Release (FR-9.2, 9.3): the updater (plugin, Roost's update commands and prompt), the
      release workflow through Roost's `release-app.yml`, an updater key pair, RELEASING.md.
- [x] 8.9 OAuth 2 tokens (FR-5.2, D-H21): a sign-in can be named, several tokens are kept per
      configuration, and "Manage tokens" switches the one in use, renames and deletes them.

## Notes

- 8.3: completion works in the URL, header values and query values; the body editor does not
  complete yet. In the URL, header and query inputs each `{{variable}}` is highlighted (red when
  undefined) by a layer drawn over the input, and hovering it shows a tooltip with its value
  (secrets masked) and source. The row of variables under the URL shows the same tooltip on
  hover or focus, and covers the body. It is a React tooltip, not `title`: with `title` no value
  could be seen in the app.

- 8.7: XML `nodeType` (3.2) is not shown yet; XML bodies are rare in the specs seen so far.

## Acceptance

A signed release installs on macOS and Linux and updates itself to the next one. This needs the
user: the updater private key and signing certificates as repository secrets, and two tagged
releases.
