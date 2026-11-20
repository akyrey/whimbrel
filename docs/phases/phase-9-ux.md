# Phase 9: Everyday UX

Goal: small changes that make daily use faster: finding requests, reading responses, knowing
where a request goes, and more of it from the keyboard.

Design: [PRD.md](../PRD.md) FR-1, FR-3, FR-4.6, FR-9.1; D-H17 (JSONPath).

## Tasks

- [x] 9.1 Light theme: every colour class comes from a scale the light theme mirrors (zinc,
      sky, rose, amber, emerald from `@roost/ui/theme-light.css`; violet from `index.css`); a
      test fails on any other scale.
- [x] 9.2 Tree filter: a box above the tree keeps requests whose name, method or file path
      contains the text (case-insensitive), with their folders; examples are hidden while
      filtering; Escape clears it.
- [x] 9.3 Response body: "Copy body" and a wrap toggle next to "Save to file".
- [x] 9.4 JSONPath filter: an RFC 9535 expression above a JSON body shows the matches as a
      pretty JSON array; an invalid expression shows the parser's message. The query runs in
      `wbl-core` with the same engine as captures.
- [x] 9.5 Shortcuts: next and previous tab, duplicate the tab's request, filter the tree, beautify
      the body, toggle the response beside/below; each is also a palette command.
- [x] 9.6 Environment chip: the request bar shows the collection's active environment (none,
      a name, or a protected name in rose with a lock); clicking it focuses the picker.
- [x] 9.7 Empty state: with no tab open, actions to open a collection, import
      OpenAPI / Postman / cURL, and start a new request, with their shortcuts where they have one.

## Notes

- 9.2: the tree's nodes carry no URL, so the filter matches the file path instead; a folder
  whose name matches keeps all of its contents.

- 9.5: next/previous tab are ⌃Tab / ⌃⇧Tab (or ⌘⌥→ / ⌘⌥←), duplicate ⌘⇧D, filter ⌘P, beautify
  ⌘⇧B, response placement ⌘J. CodeMirror's basic setup binds ⌘D, ⌘[ and ⌘], and the registry
  cannot match ⇧ with brackets, so those are avoided. A duplicate is a new unsaved tab.
- 9.7: creating a collection needs the tree's name dialog, so it stays in the sidebar.

## Acceptance

Each task's tests pass; the frontend keeps 80% line coverage; `cargo test -p wbl-core` passes.
