# Roadmap

Nine phases (0–8). Each ends with something runnable. As in Dunlin, each phase gets a brief in
`docs/phases/` (tasks, tests, acceptance criteria) before work on it starts; the task lists below
are the seeds for those briefs.

## Status

- [ ] Phase 0: Extract the shared kit from Dunlin — 0a–0d done locally; waiting on publishing
      `akyrey/roost` and `akyrey/whimbrel` and refreshing the lockfiles (see "Publishing" below)
- [x] Phase 1: Skeleton — send a request ([brief](phases/phase-1-skeleton.md))
- [ ] Phase 2: Collections on disk
- [ ] Phase 3: Environments, variables and secrets
- [ ] Phase 4: Responses, history, streaming
- [ ] Phase 5: Auth
- [ ] Phase 6: OpenAPI import and spec browser
- [ ] Phase 7: Chaining, assertions, runner, re-import
- [ ] Phase 8: Polish, interop, release

## Order and dependencies

```
0 ──► 1 ──► 2 ──► 3 ──► 5
            │     │
            │     └──► 4
            └──► 6 (needs 2; auth mapping finishes after 5) ──► 7 ──► 8
```

Phase 0 must not change Dunlin's behaviour: Dunlin's full test suite and E2E run green after
each extraction. After Phase 0, a change to the kit lands in Roost and is tagged, and then
each app bumps the tag on its own schedule. During Whimbrel phases 1–4 the kit will churn, so
develop against a linked local checkout and tag Roost at the end of each phase. Phase 6 can start in parallel with 3–5 because `wbl-openapi` is pure.

## Publishing (the rest of Phase 0)

Done locally, not pushed:

- Dunlin, branch `refactor/roost-seams`: the kit extracted inside Dunlin behind `@roost/*`
  aliases and `roost-*` crates (0a). Self-contained: it builds and passes as is.
- Roost, `../roost`, tag `v0.1.0`: the kit with its history (0b).
- Dunlin, branch `chore/use-roost` (on top of 0a): Dunlin on the Roost tag (0c). Its lockfiles
  can only be refreshed once Roost is on GitHub.
- Whimbrel, this repository (0d), linked to `../roost` for development; no lockfiles committed.

To finish: create `akyrey/roost` (public, so the reusable workflow and git dependencies need no
token) and push `main` and `v0.1.0`; then in Dunlin (`chore/use-roost`) and here run
`scripts/unlink-roost.sh`, `cargo update -p roost-shell -p roost-keychain` and
`pnpm --dir apps/desktop install`, and commit the lockfiles; create `akyrey/whimbrel` and push.
Check that an installed Dunlin updates to a release built through Roost's workflow.

## Phases

### Phase 0 — Create Roost and move Dunlin onto it (D-H09)

- 0a. **Seams first, in Dunlin.** Inside Dunlin, cut the seams listed in ARCHITECTURE.md so
  each unit no longer imports Dunlin stores: props, callbacks, a generic `CellValue`, and app
  registration of palette commands. One unit per PR, with Dunlin's suite as the gate. This
  is the risky part, and doing it before any repository change keeps it easy to review.
- 0b. **Create `akyrey/roost`** (public, so the reusable workflow and git dependencies need no
  tokens). Bring the extracted files over *with their history* (`git filter-repo --path …`
  on a clone of Dunlin). Set up Cargo and pnpm workspaces, CI (fmt, clippy, tests, coverage,
  typecheck, lint), `release-app.yml` as a `workflow_call` workflow, and the
  macOS signing-certificate script. Tag `v0.1.0`.
- 0c. **Switch Dunlin to Roost.** Replace the moved code with dependencies on the Roost tag.
  Make Dunlin's `release.yml` a thin caller of `release-app.yml`. Add the `link-roost`
  scripts. Dunlin's updater endpoint is unchanged, because it still releases from its own
  repository.
- 0d. **Create `akyrey/whimbrel`** from this directory (`git init`, then push), with these
  docs, a CLAUDE.md adapted from Dunlin's, and CI that depends on the Roost tag.
- **Accept:** Dunlin's tests, coverage and E2E are unchanged. A Dunlin release built through
  Roost's workflow installs, and an older installed Dunlin updates to it. Roost CI is green.
  Whimbrel CI builds an empty shell that uses `@roost/ui`.

### Phase 1 — Skeleton

- Workspace, `wbl-core` request model, `wbl-http::send` with cancel, Tauri app using the kit
  (window, palette, theme, settings, updater wired but disabled).
- Request editor: method, URL, headers, raw body. Response: status, headers, pretty JSON.
- Spike: timing breakdown via reqwest vs hyper (record the outcome as a decision).
- **Accept:** send GET/POST against a local test server; cancel works; app starts < 1 s.

### Phase 2 — Collections on disk

- `wbl-format`: read/write TOML, deterministic writer, round-trip property tests
  (parse → write → parse equal; unchanged save → byte-identical).
- Collection tree with folders, create/rename/move/duplicate/delete, ordering.
- File watcher with self-write suppression; conflict prompt for dirty tabs; unparsable files
  shown as errors and never overwritten.
- Tabs persist across restarts.
- **Accept:** edit in the app, `git diff` shows only the intended lines; `git checkout` of
  another branch updates the tree live.

### Phase 3 — Environments, variables and secrets

- Interpolation engine with scopes and precedence, dynamic values, autocomplete and hover.
- Environment files, `.local/` overrides, `.gitignore` management.
- Secrets in the keychain; missing-secret check before send; redaction via secret spans in
  preview, logs, cURL.
- Protected environments with confirmation.
- Saved examples (FR-2.8): overlay model and merge in `wbl-core`, `[[examples]]` read/write in
  `wbl-format`, tree entries, apply / send / open-in-tab, a diff against the active example,
  palette commands, the secret-value guard on save.
- **Accept:** a repo-wide grep after a session finds no secret values on disk outside the
  keychain (automated test with a sentinel secret, including one pasted into an example).
  Changing a base header reaches every example that does not override it.

### Phase 4 — Responses, history, streaming

- Viewers: pretty/raw/hex/image, JSON-array-as-grid, large-body spill to file, SSE and JSON
  Lines incremental view.
- Timings, cookies jar per collection, redirects, proxy, TLS settings, client certs.
- `wbl-store` history with redaction and size caps; history panel with search.
- Copy as cURL / import from cURL.
- **Accept:** 10 MB JSON viewable without UI freeze (E2E perf test like Dunlin's grid one).

### Phase 5 — Auth

- Basic, Bearer, API key, inheritance through folders.
- OAuth 2: client credentials, auth code + PKCE (loopback redirect, system browser), device
  flow, refresh; tokens in the keychain.
- **Accept:** each flow against a local test IdP (e.g. a containerised Keycloak, `#[ignore]`
  integration tests like Dunlin's Docker ones).

### Phase 6 — OpenAPI import and spec browser

- Spike `roas` vs own model on the fixture set (D-H05); record the outcome.
- Parsers for 2.0, 3.0, 3.1, 3.2; `$ref` resolution (internal, files, URLs); normalised model;
  upconversion.
- Example synthesis from JSON Schema 2020-12.
- Generator to collection files (tags → folders, servers → environments, security → auth).
- Spec browser with schema tree and two-way links.
- **Accept:** fixtures for every row of the OPENAPI.md feature table import with snapshot tests;
  Petstore 2.0/3.0/3.1/3.2 and three large public specs import under NFR-3.

### Phase 7 — Chaining, assertions, runner, re-import

- Captures (JSONPath/header/regex), assertions, folder runner with results grid.
- Response validation against the operation schema.
- Re-import with three-way diff and per-field accept.
- **Accept:** login → capture token → authorised call works as a run; re-importing a changed
  spec keeps user edits.

### Phase 8 — Polish, interop, release

- Import Postman v2.1 and `.http`; export `.http`.
- Shortcuts, palette commands for everything, settings, restore.
- Packaging via the shared workflow; new updater key pair; signing per D-037.
- **Accept:** a signed release installs on macOS and Linux and updates itself to the next one.

## Requirement coverage

| Requirement | Phase |
|---|---|
| FR-1 Collections on disk | 2 |
| FR-2 Request editor | 1, 2, 3 (variables, saved examples), 4 (cURL), 6 (examples from specs) |
| FR-3 Responses | 1, 4 |
| FR-4 Environments and secrets | 3 |
| FR-5 Auth | 5 |
| FR-6 Chaining | 7 |
| FR-7 OpenAPI | 6, 7 (validation, re-import) |
| FR-8 History | 4 |
| FR-9 Shared shell | 0, 1, 8 |

## Risks to retire early

1. **Extraction breaks Dunlin** — move one unit per PR, keep Dunlin's suite as the gate (Phase 0).
1b. **Cross-repo friction** — kit and app changes that depend on each other need two PRs and a
   tag. Mitigations: the local link scripts, small kit releases, and keeping app-specific code
   out of the kit (if only one app uses something, it stays in that app).
2. **Deterministic, comment-preserving writes** — property tests in Phase 2 before any UI depends
   on them.
3. **OpenAPI breadth** — fixture set assembled at the start of Phase 6, spike before building.
4. **Secret leakage** — sentinel-secret test in Phase 3 kept running in every later phase.
5. **Timing breakdown** — reqwest may not expose it; decide in Phase 1.

## Release and signing

- Reuse Dunlin's self-signed code-signing certificate approach (D-037); a separate certificate
  name per app is cleaner but not required.
- Generate a **new** Tauri updater key pair for this app (`pnpm tauri signer generate`); do not
  share Dunlin's, so one leaked key cannot ship updates to both apps.
