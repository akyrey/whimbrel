# Product requirements

Product name: **Whimbrel** (D-H01). Crate prefix `wbl-`, bundle id `dev.whimbrel.desktop`.

## Problem

Postman, Insomnia and similar tools want an account, sync collections to their cloud, store
secrets next to requests, and start slowly. Collections live in an opaque store, so they cannot be
reviewed in a pull request or versioned with the code they exercise.

The goal is a small, fast desktop HTTP client whose collections are **plain files in a git
repository**, whose secrets **never touch those files**, and which needs **no account and no
network service of its own**.

## User

A developer on their own machine who already keeps API code in git, works against local,
staging and production environments, and often receives an OpenAPI document instead of a
collection.

## Goals

1. Requests, folders and environments are human-readable files that diff and merge cleanly.
2. Secrets live in the OS keychain only; a committed collection never contains one.
3. Import any OpenAPI document (Swagger 2.0, OpenAPI 3.0.x, 3.1.x, 3.2.x) as a collection and
   browse the spec itself inside the app.
4. Start in about a second; stay responsive on multi-megabyte responses.
5. Reuse Dunlin's editor, grid, command palette, updater and signing so the shell costs little.

## Non-goals for v1

- Accounts, cloud sync, team workspaces, sharing links. Git is the sharing mechanism.
- Running git (commit, pull, push). The app reads and writes files; the user's git does the rest.
  It reloads when files change on disk.
- Pre-request / test scripts in JavaScript. Chaining is done declaratively (FR-6).
- Mock servers, monitors, load testing, API design/editing of OpenAPI documents.
- GraphQL, gRPC, WebSocket, MQTT. (HTTP/1.1 and HTTP/2 only; SSE responses are displayed.)
- Windows builds (same as Dunlin; revisit after v1).

## Functional requirements

### FR-1 Collections on disk

- FR-1.1 A collection is a directory containing `collection.toml`. Folders are directories;
  requests are `*.req.toml` files. See [FILE_FORMAT.md](FILE_FORMAT.md).
- FR-1.2 Open several collections at once (a workspace remembers which).
- FR-1.3 Create, rename, move, duplicate and delete requests and folders; the app writes the
  files, preserving order through an explicit `order` key, not file names.
- FR-1.4 Writes are deterministic (stable key order, stable formatting, LF, trailing newline) so
  saving an unchanged request produces no diff.
- FR-1.5 External changes (git checkout, pull, another editor) are picked up by a file watcher.
  An open request with unsaved edits that changes on disk prompts: keep mine / take theirs.
- FR-1.6 Files the app cannot parse are shown as errors in the tree with the parser message;
  they are never overwritten.

### FR-2 Request editor

- FR-2.1 Method (including custom methods and `QUERY`), URL, query parameters, headers, body,
  auth, per-request settings (timeout, redirects, TLS verification).
- FR-2.2 Body kinds: none, raw (JSON, XML, text, with highlighting from Dunlin's editor),
  form-urlencoded, multipart (fields and files), binary file.
- FR-2.3 Query parameters and URL stay in sync both ways.
- FR-2.4 `{{variable}}` interpolation everywhere, with autocomplete and an inline hover showing
  the resolved value (secrets masked).
- FR-2.5 "Resolved preview": the exact request that will be sent, secrets masked, before sending.
- FR-2.6 Copy as cURL (secrets masked unless explicitly revealed); import from cURL.
- FR-2.7 Multiple request tabs; unsaved tabs survive a restart.
- FR-2.8 **Saved examples.** Save the current inputs of a request (path params, query,
  headers, body, request variables) under a name such as "Example A". Each example stores only
  what differs from the base request (D-H08).
  - FR-2.8.1 From an example: apply it to the editor, send with it directly, or open it in a
    new tab.
  - FR-2.8.2 Rename, duplicate, delete, reorder and update an example ("save changes to
    Example A"). Unsaved changes made while an example is active are shown as a diff
    against that example.
  - FR-2.8.3 Examples show under their request in the tree and in the command palette
    ("Send Create user › Example A").
  - FR-2.8.4 Optionally store a response with an example as a reference. It is shown next to
    the live response for comparison and is never sent anywhere.
  - FR-2.8.5 Examples hold `{{secret}}` references, never secret values. Saving an example
    whose literal text matches a known secret value is refused, with the reason shown.

### FR-3 Responses

- FR-3.1 Status, time, size, headers, cookies, and timing breakdown where available.
- FR-3.2 Body viewers: pretty JSON / XML / HTML source, raw, hex for binary, image preview.
- FR-3.3 JSON arrays of objects can be shown in Dunlin's data grid.
- FR-3.4 Server-Sent Events and `application/jsonl` streams are shown incrementally.
- FR-3.5 Bodies larger than a configurable cap (default 10 MB) stream to a temp file; the viewer
  shows the head and offers "save to file".
- FR-3.6 Cancel an in-flight request.

### FR-4 Environments and secrets

- FR-4.1 Environments are files in the collection (`environments/<name>.toml`) holding
  non-secret variables and the **names** of secret variables.
- FR-4.2 Secret values are stored in the OS keychain, keyed by collection id, environment and
  variable name. A missing secret is reported before sending, not sent as empty.
- FR-4.3 Optional per-user, git-ignored overrides (`.local/<env>.toml`); the app adds the
  `.gitignore` entry when it creates a collection.
- FR-4.4 Variable precedence: request > folder > environment local override > environment >
  collection. Documented and shown in the resolved preview.
- FR-4.5 Built-in dynamic values: `{{$uuid}}`, `{{$timestamp}}`, `{{$isoTimestamp}}`,
  `{{$randomInt}}`.
- FR-4.6 Environments can be marked **protected** (e.g. production): sending asks for
  confirmation, like Dunlin's dangerous-statement guard.
- FR-4.7 Secret values never appear in logs, history files, crash reports or copied cURL.

### FR-5 Auth

- FR-5.1 None, inherit (from folder/collection), Basic, Bearer, API key (header or query).
- FR-5.2 OAuth 2.0: client credentials, authorization code with PKCE (system browser + loopback
  redirect), device authorization; tokens cached in the keychain and refreshed. Several named
  tokens can be kept per configuration; one is in use, and they can be renamed, switched and
  deleted (D-H21).
- FR-5.3 Client certificates (PEM / PKCS#12) and custom CA per collection.

### FR-6 Chaining without scripts

- FR-6.1 A request may declare **captures**: JSONPath / header / regex extractions from the
  response into a runtime variable (optionally persisted to the keychain if marked secret).
- FR-6.2 A request may declare simple **assertions** (status, header, JSONPath equals/exists,
  response time below), shown as pass/fail.
- FR-6.3 Run a folder in order (a "run"), stopping on first failure or continuing; results in
  a grid. A step can be a request with a chosen saved example, and a step can run one request
  once per example.

### FR-7 OpenAPI

See [OPENAPI.md](OPENAPI.md).

- FR-7.1 Import Swagger 2.0, OpenAPI 3.0.x, 3.1.x and 3.2.x documents, JSON or YAML, from a
  file or URL.
- FR-7.2 Resolve `$ref` within the document, to relative files, and (with confirmation) to URLs;
  handle cycles.
- FR-7.3 Generate a collection: folder per tag (nested tags in 3.2), request per operation,
  servers → `baseUrl` per environment, security schemes → auth, example bodies.
- FR-7.4 Spec browser: operations, parameters, request/response schemas and examples rendered
  readably, linked both ways to generated requests.
- FR-7.5 Re-import an updated spec: show added / changed / removed operations and apply them
  without overwriting fields the user edited.
- FR-7.6 Optionally validate a response against the operation's response schema.

### FR-8 History

- FR-8.1 Every sent request and its response metadata is recorded locally (outside the
  collection, in the app's SQLite), searchable, with secret values redacted.
- FR-8.2 Response bodies in history are kept up to a size cap; history can be cleared.

### FR-9 Shared shell (from Dunlin)

- FR-9.1 Command palette, rebindable shortcuts, light/dark theme, settings.
- FR-9.2 Signed in-app updates from GitHub releases.
- FR-9.3 Signed macOS app; Linux deb, rpm, AppImage.
- FR-9.4 Restores window, open collections and tabs on start.

## Non-functional requirements

- NFR-1 Cold start under ~1 s on a recent laptop; first request sent in under 50 ms overhead.
- NFR-2 A 10 MB JSON response is viewable without freezing the UI.
- NFR-3 A 5 MB OpenAPI document imports in under 3 s.
- NFR-4 No telemetry. No network traffic except user requests, explicit spec URL fetches and the
  update check (which can be disabled).
- NFR-5 80% line coverage per crate and for the frontend.
