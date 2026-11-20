# Architecture

Same shape as Dunlin (D-001 there): Tauri 2 shell, Rust workspace backend, React + TypeScript
frontend, types generated with ts-rs, all I/O in Rust.

## Reuse from Dunlin

Dunlin's reusable pieces are today coupled to its database features (the editor imports
`sessionsStore` and `schemaStore`, the palette imports `editorStore` and `schemaStore`, the
updater prompt imports `editStore`). Phase 0 extracts them behind small interfaces into the
shared **Roost** repository (D-H09), and Dunlin then depends on Roost without any change in
behaviour.

### Three repositories

```
akyrey/roost      shared kit: crates/roost-*, packages/@roost/*, reusable release workflow
     ▲    ▲         tagged vX.Y.Z; its own CI runs every kit test
     │    │
     │    └──── akyrey/dunlin     depends on roost by tag (Cargo git dep + pnpm git dep)
     └───────── akyrey/whimbrel   depends on roost by tag; this repository
```

Cargo: `roost-shell = { git = "https://github.com/akyrey/roost", tag = "v0.1.0" }` (Cargo
finds a crate by name in a git workspace). pnpm:
`"@roost/editor": "github:akyrey/roost#v0.1.0&path:/packages/editor"`. **Verify** the
manifest form, and that pnpm resolves `@roost/*` references *between* kit packages when they
are installed from git (otherwise each app lists every `@roost/*` package it uses, transitively).

Frontend packages ship TypeScript source (`exports` → `src/index.ts`); the app's Vite compiles
them, so the kit has no build step. React, CodeMirror, Zustand and TanStack Virtual are
`peerDependencies`, so each app has one copy of each.

Working across repos: a git-ignored `.cargo/config.toml` with
`[patch."https://github.com/akyrey/roost"]` pointing at a local checkout, and
`pnpm.overrides` with `link:../roost/packages/*` in a git-ignored local file or a script
(`scripts/link-roost.sh` / `unlink-roost.sh`). CI always uses the tag.

| Shared unit | From Dunlin | Becomes | Seam to cut |
|---|---|---|---|
| Code editor | `features/editor/CodeEditor.tsx`, `theme.ts`, `appearance.ts`, `languages.ts` | `@roost/editor` | language + completion source passed in as props, no store imports |
| Data grid | `features/grid/*` (DataGrid, selection, widths, copy, present, CellViewer) | `@roost/grid` | cell value model as a generic `CellValue` adapter; editing hooks optional |
| Command palette | `features/palette/CommandPalette.tsx`, `match.ts`, `commands.ts` | `@roost/palette` | app registers commands; palette knows no stores |
| Shortcuts | `features/shortcuts/registry.ts`, `ShortcutsView.tsx` | `@roost/shortcuts` | already almost free-standing |
| Base components | `components/*` (Button, Modal, ContextMenu, Toaster, ConfirmDialog, StatusBar, PanelBoundary) | `@roost/ui` | none |
| Settings / theme | `features/settings/theme.ts`, `theme-light.css`, `index.css` tokens | `@roost/ui` | settings schema is per app |
| Updates UI | `features/updates/*` | `@roost/updates` | "unsaved work?" becomes a callback |
| Workspace chrome | `TabStrip.tsx`, `SidebarResizer.tsx`, `layout.ts` | `@roost/ui` | tab model generic over payload |
| Updater backend | `src-tauri/src/updates.rs` | `roost-shell` crate | none |
| Logging | `src-tauri/src/logging*` | `roost-shell` crate | redaction list provided by app |
| Keychain | keychain part of `dbc-store` | `roost-keychain` crate | service name per app |
| Release / signing | `.github/workflows/release.yml`, `scripts/make-macos-signing-cert.sh`, `tauri.*.conf.json` | `akyrey/roost/.github/workflows/release-app.yml@vX` | app name, bundle id, updater pubkey as inputs; secrets passed with `secrets: inherit` |

Tests move with the code: the kit's unit and component tests run in Roost's CI. Each app keeps
its integration and E2E tests, which exercise the kit as the app uses it.

## Workspace layout (the Whimbrel repository)

```
crates/wbl-core          request/collection/environment model, saved-example overlays,
                        interpolation, no I/O
crates/wbl-format        read/write collection files (TOML), deterministic writer, migrations
crates/wbl-http          send requests: reqwest/hyper, TLS, proxies, cookies, streaming, timings
crates/wbl-auth          auth schemes, OAuth 2 flows, token cache (via roost-keychain)
crates/wbl-openapi       parse 2.0/3.0/3.1/3.2, resolve refs, normalise, generate, diff
crates/wbl-interop       Postman v2.1 and .http in, .http out
crates/wbl-store         app SQLite: history, workspace state, settings (not collections)
crates/wbl-engine        the app's operations without Tauri: scopes, secrets, auth, sending,
                        checks, runner, spec and Postman/.http reading (D-H23)
crates/wbl-mcp           MCP server over stdio for AI tools, run as `whimbrel mcp` (D-H23)
src-tauri               Tauri commands, file watcher, in-flight request registry (thin)
src                     React frontend by feature: collections, request, examples,
                        response, env, openapi, history, runner
```

Dependency rule: `wbl-core` depends on no workspace crate. `wbl-format`, `wbl-http`, `wbl-auth`,
`wbl-store` depend only on `wbl-core` (and `roost-*`). `wbl-openapi` and `wbl-interop` depend on `wbl-core` and
`wbl-format` (D-H16, D-H19): it generates stored requests and environments and writes them into a
collection with `wbl-format`'s operations. `wbl-engine` depends on `wbl-core`, `wbl-format`, `wbl-http`,
`wbl-auth`, `wbl-openapi`, `wbl-interop` (and `roost-*`); `wbl-mcp` on the `wbl-*` crates it serves (D-H23). Only `src-tauri`
depends on everything.

## Data flow: sending a request

```
editor state ──► wbl-core::apply_example(request, example?)   (pure; returns a new request)
             ──► wbl-core::resolve(request, scopes, secrets) ──► ResolvedRequest
                         │ unresolved var / missing secret → error before any I/O
                         ▼
                 wbl-auth::apply(resolved, auth)  (may run an OAuth flow)
                         ▼
                 wbl-http::send(resolved, settings, cancel) ──► stream of events
                         │   Started{timings} · Headers · BodyChunk · Done | Failed
                         ▼
                 Tauri channel to the frontend; wbl-store records redacted history
```

`ResolvedRequest` carries a list of secret spans so every consumer (preview, history, cURL
export, logs) can redact without knowing where secrets came from.

## Data flow: files

```
disk ◄──► wbl-format ◄──► src-tauri collection service ◄──IPC──► frontend stores
  ▲                              │
  └── notify watcher ────────────┘ (debounced; self-writes suppressed by content hash)
```

The backend is the source of truth for file contents. The frontend holds drafts; saving sends
the whole request and gets back the canonical form.

## Key technology choices (to verify in Phase 1, per Dunlin's "Verify" convention)

- HTTP: `reqwest` with `rustls`, HTTP/2, `cookie_store`, streaming bodies. **Verify** whether
  DNS/connect/TLS timings are reachable through reqwest; if not, use `hyper-util` directly
  for the connector and keep reqwest out.
- TOML: `toml_edit`, so rewriting a file preserves comments and the user's formatting.
- YAML for OpenAPI: `serde_yaml` successor (`serde_yml` / `serde_norway`) — **Verify** which is
  maintained.
- OpenAPI model: see D-H05.
- JSONPath: `serde_json_path` (RFC 9535).
- JSON Schema validation: `jsonschema` crate (supports 2020-12, needed for 3.1/3.2).
- File watching: `notify` + `notify-debouncer-full`.
- Keychain: `keyring` (as in Dunlin).
