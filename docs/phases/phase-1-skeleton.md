# Phase 1: Skeleton

Goal: the app starts, shows a request editor built from the Roost kit, sends a request and shows
the response. Proves the shell, the IPC shape, the HTTP engine and cancellation.

Design: [ARCHITECTURE.md](../ARCHITECTURE.md), [PRD.md](../PRD.md) FR-2, FR-3, FR-9.

## Tasks

- [x] 1.1 Workspace: `wbl-core`, `wbl-http`, `apps/desktop/src-tauri`; Roost by tag with the link
      scripts; lints and toolchain as Dunlin's.
- [x] 1.2 `wbl-core` request model: method (any RFC 9110 token, so `QUERY` and custom methods
      work), URL (http/https), headers (ordered, duplicates kept, can be disabled), raw body
      (JSON, XML, text). `validate` rejects what could not be sent, with a message per field.
- [x] 1.3 `wbl-http::send`: builds the request, enforces a timeout, follows redirects (limit 10),
      decompresses, reads the body up to a cap (marks it truncated past it), measures total time and
      time to headers. Cancellable. Errors carry stable codes: `invalid_request`, `connect`,
      `tls`, `timeout`, `redirect`, `body`, `cancelled`, `http`.
- [x] 1.4 Spike: can reqwest report DNS / connect / TLS timings? Outcome in D-H10.
- [x] 1.5 Tauri commands `request_send` and `request_cancel` with an in-flight registry; logging
      through `roost-shell` (command, duration, error code only); `AppError` from every error.
- [x] 1.6 Generated TypeScript types (ts-rs), checked in CI as in Dunlin.
- [x] 1.7 Frontend: request editor (method, URL, headers table, body editor from `@roost/editor`),
      Send / Cancel, response panel (status, time, size, headers, pretty JSON or text), command
      palette and shortcuts from `@roost/palette` and `@roost/shortcuts`, toasts from `@roost/ui`.
- [x] 1.8 E2E against a mocked backend: send, see the response, cancel, see an error.

## Acceptance

- Send GET and POST with a JSON body to a local test server; the response shows status,
  headers and the pretty-printed body.
- Cancelling an in-flight request ends it with `cancelled` and the UI is ready again.
- `cargo clippy -D warnings`, all tests, 80% coverage per crate and frontend.
- Start-up under a second (measured in Phase 8 with the release build, as in Dunlin).

## Not in this phase

Collections on disk (Phase 2), variables and secrets (Phase 3), the updater (Phase 8: Whimbrel
needs its own updater key pair first), query-parameter table sync (Phase 2).
