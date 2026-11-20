# Phase 12: MCP server for AI tools

Goal: AI tools manage collections and environments, and send requests, through a Model Context
Protocol server that ships in the app binary as `whimbrel mcp`, without ever seeing a secret.

Design: [PRD.md](../PRD.md) FR-10; [DECISIONS.md](../DECISIONS.md) D-H23 (and D-H04, D-H12 for
secrets); [ARCHITECTURE.md](../ARCHITECTURE.md) (workspace layout, dependency rule);
[FILE_FORMAT.md](../FILE_FORMAT.md).

## Before starting

- [x] **Verified** `rmcp` 3.11: `#[tool_router]` / `#[tool_handler]` macros, stdio transport,
  an in-process client over `tokio::io::duplex`. Elicitation and roots depend on the protocol
  version (D-H23), so neither is used.
- **Verify** that the bundled executable (`Whimbrel.app/Contents/MacOS/Whimbrel` as the README
  says) runs `mcp` headless from a terminal, and that the AppImage and deb binaries take
  arguments the same way. The debug binary does (`target/debug/whimbrel mcp`, task 12.10).

## Tasks

### Shared pipeline

- [x] 12.1 `wbl-engine` crate: move `resolve.rs`, `secrets.rs`, `auth_apply.rs`, `checks.rs`
      and the runner core out of `src-tauri` with their tests, replacing Tauri types (channels,
      `AppError`) with the engine's own error enum (stable codes) and plain callbacks or
      streams. `src-tauri` commands call the engine. Behaviour unchanged: the existing
      `src-tauri` and frontend suites, including the sentinel-secret test, are the gate.
- [x] 12.2 `wbl-engine::deliver::send_once`: resolve, apply auth (cached or refreshed OAuth 2
      token only; without one, `oauth_required`), send with `wbl-http`, and return a masked
      request, the response cut to 256 kB, and the checks. Every known secret value (used,
      in the environment, just captured) is masked as history masks it; history and the
      server share `wbl-engine::mask`. Tests against a local server, including a response
      that echoes a secret.

### Server

- [x] 12.3 `wbl-mcp` crate skeleton: server info, the set of reachable collections from
      `--collection` arguments (a directory with a `collection.toml`), named by folder or
      name. Another collection fails with `not_reachable`, a path escaping one with
      `invalid_path`. Errors map to MCP tool errors carrying the stable code and a message
      without URLs or values (D-H11). Tests through an in-process client.
- [x] 12.4 Read tools: `list_collections`, `get_tree` (folders and requests with method, name,
      path, order), `get_request` (the file's request, with examples, captures and assertions;
      inherited headers and auth listed separately), `list_environments`, `get_environment`
      (variables with values, secret ones as names with `secret: true`, `protected`).
- [x] 12.5 Request and folder tools: `create_request` (fields left out take defaults),
      `update_request` (the fields given replace the current ones; refuses literal secret
      values with the app's guard), `create_folder`, `rename`, `move`, `reorder`, `duplicate`,
      `delete`, and `set_headers` / `set_auth` for folders and the collection (guarded the same
      way). Each returns the canonical form.
      Tests: an unchanged update leaves the file byte-identical; a move keeps the tree valid.
- [x] 12.6 Environment tools: `create_environment`, `set_variables` (add, change, remove;
      marking a variable secret stores the name only and removes any value from the file),
      `set_collection_variables`, `delete_environment` (the file and `.local`; keychain values
      stay, so restoring the file from git restores them). No tool sets a secret value or
      changes whether an environment is protected. The
      sentinel test: after every write tool, no secret value appears on disk or in any result.
- [x] 12.7 Import tools: `import_openapi` (file path or URL), `import_file` (Postman
      collection or environment, `.http`), `import_curl` (into a folder), and `export_http`
      (returns the text; writes nothing), through `wbl-openapi` and `wbl-interop`, returning
      the import report (what was skipped and why). Reading specs and files moves from
      `src-tauri` into `wbl-engine` so the app and the server share it.
- [x] 12.8 `send_request` (a stored request, optional environment and example) and
      `run_folder` (captures and assertions, returns the results grid as rows). A protected
      environment is refused with `protected_environment`. Response bodies are text up to 256 kB; binary bodies are
      described (type, size), not returned.

### Shipping

- [x] 12.9 `whimbrel mcp [--collection <dir>]…` in `main.rs`: parse arguments before Tauri
      starts and run the server on a tokio runtime. Nothing is logged in this mode; messages
      for people go to stderr (the end-to-end test checks stdout holds only protocol
      messages). `whimbrel mcp --help` prints usage; bad arguments exit with 2.
- [x] 12.10 End to end (`src-tauri/tests/mcp.rs`): a client process starts the built binary,
      creates an environment and a request, sends it to a local server, and checks every
      stdout line is a JSON-RPC message. It uses no secret, so it never touches the real
      keychain; the sentinel secret is covered in-process (`wbl-engine::deliver`,
      `wbl-mcp` send and environment tests). A running app's watcher picks up the new file
      (covered by the existing watcher tests for outside writes).
- [x] 12.11 Docs: a "Use with AI tools (MCP)" section in the README with the configuration
      for Claude Desktop, Claude Code and Cursor, the tool list, and what the server never
      does (secret values, protected environments, sign-in, history).

## Notes

- Tool inputs have JSON Schemas generated from Rust types (`schemars` 1, as `rmcp` expects).
  Requests, headers and auth are taken as JSON in the shape `get_request` returns and parsed
  into the `wbl-core` types before any file is touched, so a malformed one fails with
  `invalid_argument`.
- Tool descriptions are written for the model: say what a tool changes on disk and that
  secrets are names only.
- Concurrency with the running app needs nothing new: writes are atomic (`write_atomic`) and
  the watcher treats them like a `git checkout`.
- Out of scope: HTTP transport, MCP resources and prompts, live app state (open tabs, the
  active environment), recording MCP sends in history. Revisit after use.

## Acceptance

Each task's tests pass; `wbl-engine`, `wbl-mcp` and `src-tauri` keep 80% line coverage; the
sentinel-secret test covers MCP results; the app behaves as before (existing suites and E2E
green); the end-to-end test in 12.10 passes against the built binary on macOS and Linux.
