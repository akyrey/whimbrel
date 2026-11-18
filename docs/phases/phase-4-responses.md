# Phase 4: Responses, history, streaming

Goal: every kind of response is readable (text, JSON as a table, binary, images, streams, very
large bodies), requests have their own settings, cookies persist per collection, sent requests
are kept in a searchable history without their secrets, and cURL goes both ways.

Design: [PRD.md](../PRD.md) FR-2.6, FR-3, FR-8; D-H10, D-H11, D-H12.

## Tasks

- [x] 4.1 Bodies: kept in memory up to 10 MB; a longer one is written whole to a temporary
      file and its first 1 MB is shown, with "Save to file". Binary bodies come as bytes (hex
      view; images previewed). Pretty and raw views; a JSON array of objects as a grid
      (`@roost/grid`).
- [x] 4.2 Request settings in the request file (`[settings]`: timeout, follow redirects, verify
      TLS) and one cookie jar per collection, kept for the session; the response lists the
      cookies it set.
- [x] 4.3 Streams: `text/event-stream` and JSON Lines (`application/jsonl`,
      `application/x-ndjson`) are shown event by event as they arrive, until the server
      closes or the user cancels.
- [x] 4.4 History (`wbl-store`, SQLite in the app's data folder, not in the collection): every
      send with its resolved request and response head, secret values masked, bodies kept up
      to 256 KB; search, open again, clear.
- [x] 4.5 cURL: copy the resolved request as a cURL command, secrets masked unless revealed
      on purpose; import a cURL command (method, URL, headers, data, basic auth) as a new
      request.
- [x] 4.6 Frontend for the above, and an E2E check that a 10 MB JSON response shows without
      freezing the window.

## Notes

- Measured (E2E, Chromium): a 10 MB JSON body shows in about 0.5 s; the longest main-thread
  task while it does is about 85 ms. Before turning off line wrapping for long bodies it was
  over a second (D-H13).
- Stream events are covered by the engine's integration test (events arrive one by one from a
  local server) and a store test; the browser E2E cannot drive a Tauri channel, so it does not
  show live events.
- A history entry's request has its secrets masked; reopened and sent as is, it would send the
  mask. The app says so when it opens one.

## Not in this phase

- DNS / connect / TLS timing breakdown (D-H10 stays).
- Proxy settings in the app: the system proxy (and `HTTP(S)_PROXY`) applies, as reqwest does
  by default.
- Client certificates and custom CAs: with auth, in Phase 5 (FR-5.3).
- Cookies across restarts, and editing the jar.
