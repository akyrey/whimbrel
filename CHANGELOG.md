# Changelog

All notable changes to Whimbrel. The format follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

## [0.6.1] - 2026-11-20

### Fixed

- Open tabs see environment changes as soon as they are saved (variables, secrets and collection
  variables); they no longer need to be closed and reopened.

## [0.6.0] - 2026-11-20

### Added

- Collections and folders can set headers that every request under them sends ("Headers…" in
  their menus). A deeper header replaces inherited ones of the same name; the request's Headers
  tab lists what it inherits, struck through where replaced.
- An OAuth 2 configuration keeps several named tokens; "Manage tokens" switches the one in use,
  renames and deletes them. A single token from an earlier version becomes the first of the list.
- The spec validation result can be hidden.

### Changed

- Opening, creating and importing (cURL, OpenAPI, Postman or .http) moved into a + menu beside the
  request filter. "Open…" and "New collection…" stay visible while no collection is open.
- The Environments editor opens as a wide dialog.

### Fixed

- The request bar stays usable in a narrow pane: it has two rows, and "Copy cURL" moved into a
  cURL menu.

## [0.5.0] - 2026-11-20

### Added

- Hovering a `{{variable}}` in the URL, a header or a query value shows its value (secrets
  masked) and where it comes from; variables are highlighted there, red when undefined. The row
  of variables under the URL shows the same tooltip, which also covers the body.

## [0.4.2] - 2026-11-20

### Fixed

- "Get token" names an empty or invalid token, authorization or device URL (for example one
  without http:// or https://) instead of failing with "builder error", and the browser opens only
  once both the authorization and token URLs are valid.
- When the authorization server cannot be reached, the error says why.

## [0.4.1] - 2026-11-20

### Fixed

- Folders have "Auth…" in their menu, so requests inside can inherit a folder's auth.
- A request set to inherit its auth lists the folders and the collection it inherits from, each
  opening that auth to edit.

## [0.4.0] - 2026-11-20

### Added

- Drag and drop in the collection tree: reorder requests and folders, move them into other
  folders or to the top of the collection; open tabs follow the moved files.
- Folders can move into other folders (with everything in them).

## [0.3.0] - 2026-11-20

### Added

- A Timing tab with a waterfall: DNS, connect (TCP and TLS), waiting and download; a reused
  connection is named as such.
- A Compare tab: the response against the one before it in the same tab (status, header changes,
  a line diff of the pretty bodies).

### Changed

- Switching to a response without the open view (cookies, events, compare) shows its body.

## [0.2.0] - 2026-11-20

### Added

- A filter above the collection tree (name, method or file path).
- A JSONPath filter over JSON responses, with the same RFC 9535 engine as captures.
- "Copy body" and a wrap toggle in the response view.
- The active environment shown beside the URL; protected ones in rose with a lock.
- Shortcuts and palette commands: next/previous tab, duplicate the request, filter the
  requests, beautify the body, put the response beside or below.
- Ways to start (new request, open a collection, imports) when no tab is open.

### Fixed

- Error text and PATCH colours keep their contrast in the light theme.

## [0.1.1] - 2026-11-20

### Added

- Requests as TOML files in git: collections, folders, saved examples, request variables.
- Environments with secrets in the OS keychain; `.local` overrides; protected environments.
- Responses: pretty, raw, table, hex and image views; streams (SSE, JSON Lines); large bodies.
- Auth: basic, bearer, API key, OAuth 2 (client credentials, authorization code with PKCE,
  device code), per-collection TLS.
- OpenAPI 2.0 to 3.2 import, a spec browser, re-import with a three-way merge, response
  validation.
- Captures, assertions, runtime variables and a folder runner.
- Form, multipart and file bodies; a query table; variable completion.
- Postman v2.1 and `.http` import, `.http` export.
- Settings, palette commands for every action, tabs restored with unsaved edits.
- Signed in-app updates from GitHub releases.
- The request's parts (params, auth, headers, body, variables, checks, settings) as tabs; the
  request/response split and the sidebar can be resized, and the sizes are remembered.
- The response can sit beside the request instead of below it (a button by the request's tabs,
  Settings, or the palette); each placement keeps its own size.
- Beautify for JSON bodies; `{{variables}}` used as bare values are kept.
