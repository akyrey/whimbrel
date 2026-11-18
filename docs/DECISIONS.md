# Decision log

Append-only, same format as Dunlin's. Entries marked **Proposed** need confirmation before
Phase 0 starts. Dunlin's decisions D-001 (Tauri 2), D-003-style tagged wire values where needed,
D-010 (ts-rs), D-011 (E2E against mocked IPC), D-025 (settings JSON), D-035/D-036/D-037
(packaging, updates, signing) apply here unless superseded.

---

## D-H01 Product name (2026-11-18)

**Outcome.** **Whimbrel** chosen. Crate prefix `wbl-`, bundle id `dev.whimbrel.desktop`,
repository `akyrey/whimbrel`.


**Context.** The repository is `postman-clone`, which cannot be the product name. A shorebird
pairs with Dunlin. Checked 2026-11-18 on crates.io (`<name>` and `<name>-core`) and GitHub:

| Name | crates.io | Notable clash |
|---|---|---|
| **Whimbrel** | free | none (largest GitHub repo has 2 stars) |
| Willet | free | a small language project, 6 stars |
| Avocet | free | `avocet-shores` Laravel packages (unrelated) |
| Sanderling | free | Arcitectus/Sanderling, an EVE Online bot API framework, 336 stars |
| Plover | free | Open Steno Project's Plover, 2.6k stars: avoid |
| Godwit, Turnstone, Curlew, Knot, Stint, Redshank, Snipe | taken | — |

**Recommendation.** Whimbrel. Crate prefix `wbl-`, bundle id `dev.whimbrel.desktop`.
**Consequences.** Bundle id, keychain service name and updater URL depend on it. Until it is
confirmed the docs use the placeholder prefix `hc-`. Check the domain and trademark before the
first public release.

## D-H02 Where the shared code lives (2026-11-18) — superseded by D-H09

**Context.** The editor, grid, palette, updater and signing must be shared, and they will change
a lot while being extracted.
**Options considered.** (a) One monorepo with both apps. (b) Pin Dunlin's repo by git tag.
(c) Copy and diverge.
**Decision.** (a): the Dunlin repository (`akyrey/dunlin`) becomes the monorepo. It keeps its
history, CI, release secrets and issues. Layout after Phase 0:

```
apps/dunlin/            (was apps/desktop)
apps/<new-app>/         this app: src-tauri + src
crates/dbc-*            Dunlin crates, unchanged
crates/desk-*           shared Rust: desk-shell (updater, logging), desk-keychain
crates/hc-*             this app's crates
packages/desk-*         shared frontend packages (pnpm workspace)
docs/                   Dunlin docs, unchanged
docs/<new-app>/         these docs, moved from postman-clone
.github/workflows/      ci.yml runs both; release-app.yml is reusable, one tag prefix per app
```

**Consequences.** Tags become `dunlin-vX.Y.Z` and `<app>-vX.Y.Z`. Each app's updater
`latest.json` must point at its own latest release, so it cannot use GitHub's
`releases/latest`. The endpoint becomes a fixed per-app release such as
`<app>-latest/latest.json`, which the release workflow updates. **Verify** in Phase 0;
installed Dunlin copies keep reading the old URL, so Dunlin's endpoint must stay valid
across the switch. The repo may be renamed later; GitHub redirects the old URL.

## D-H03 Collections are TOML, one request per file (2026-11-18)

**Context.** Candidates: `.http` (REST Client), Bruno's `.bru`, YAML, JSON, TOML.
**Decision.** TOML per request, edited with `toml_edit` to preserve comments and unknown keys.
`.http` and Postman v2.1 as import/export only.
**Consequences.** Multi-line bodies use `'''` literals; large bodies go to sidecar files.
`.http` cannot express auth inheritance, captures or assertions, which is why it is not the
primary format.

## D-H04 Secrets: names in files, values in the keychain (2026-11-18)

**Context.** The point of the product is that a committed collection is safe to push.
**Decision.** Environments list secret names; values live in the OS keychain keyed by
(collection id, environment, name). Git-ignored `.local/` overrides are allowed for
non-secret per-user values. Resolved requests carry secret spans for redaction.
**Consequences.** Cloning a collection on a new machine requires entering secrets once; the app
lists missing ones. Same Linux keyring requirement as Dunlin (D-035).

## D-H05 OpenAPI: one normalised model, parser library to be spiked (Proposed, 2026-11-18)

**Context.** Generation, browsing and re-import diffing must handle five spec lines. Few Rust
crates cover 3.2.
**Decision.** Own normalised model (3.2 superset) in `hc-openapi`. Spike `roas` for parsing; fall
back to our own serde model.
**Consequences.** Upconversion code per version, tested by fixtures.

## D-H06 No scripting engine in v1 (2026-11-18)

**Context.** Postman's JS sandbox is the main source of complexity and of untrusted code running
from a cloned repo.
**Decision.** Declarative captures and assertions only.
**Consequences.** Some Postman collections import without their scripts (reported per request).
Revisit with an embedded, sandboxed engine (e.g. `rquickjs`) only if captures prove too weak.

## D-H07 The app does not run git (2026-11-18)

**Decision.** Read and write files; watch for external changes. No commit/pull/push UI.
**Consequences.** No `git2`/`gix` dependency. A read-only "modified since HEAD" marker may come
later.

## D-H08 Saved examples are overlays stored in the request file (2026-11-18)

**Context.** Users want to save a set of request inputs ("Example A") and reuse it later. OpenAPI
documents also carry named `examples` per parameter and body.
**Decision.** A request has zero or more named examples, each an *overlay* on the base request
(variables, path params, query, headers, body), stored as `[[examples]]` in the request's own
file. Applying one never mutates the base request. Bodies over a size threshold go to a sidecar
file. Examples may reference secrets by `{{name}}` but never contain values.
**Consequences.** Changing the base request (a new header, a new URL) reaches every example
unless that example overrides it. One file per request still holds everything about it, so a
merge stays local.

## D-H09 Three repositories: Roost (shared kit), Dunlin, Whimbrel (2026-11-18)

**Context.** Supersedes D-H02. The apps should stay in separate repositories, each with its own
history, issues and releases.
**Decision.** A new public repository, `akyrey/roost`, holds the shared Rust crates
(`roost-shell`, `roost-keychain`), the frontend packages (`@roost/ui`, `editor`, `grid`,
`palette`, `shortcuts`, `updates`) and a reusable `release-app.yml` workflow. Dunlin and
Whimbrel depend on it by git tag: Cargo `git` + `tag`, and pnpm
`github:akyrey/roost#<tag>&path:/packages/<pkg>`. None of it is published to crates.io or npm.
Frontend packages ship TS source, and shared libraries are peer dependencies. Extracted
files keep their history (`git filter-repo`). The name "Roost" is not taken by any `roost-*`
crate or `@roost/*` npm package that we would use; the crate `roost` itself is taken, which
does not matter because nothing is published.
**Consequences.**
- Each app releases from its own repository, so `releases/latest/download/latest.json` keeps
  working, and the per-app updater-endpoint problem in D-H02 goes away.
- Kit changes need a Roost tag before an app's CI can use them. Local work uses the
  `link-roost` scripts.
- Kit tests run in Roost's CI, app tests in each app's. A kit change can break an app only
  when that app bumps the tag, and the bump PR's CI catches it.
- Roost follows semver. While the kit is churning (0.x), any minor bump may break the apps.
- A reusable workflow from another repository needs that repository to be public, or access
  granted from it. Roost is public.
- Updater keys stay per app (Whimbrel gets a new pair). Signing secrets are set in each app's
  repository and passed with `secrets: inherit`.

## D-H10 Phase 1 timings: time to headers and total only (2026-11-18)

**Context.** Phase 1's spike: can reqwest report DNS, connect and TLS times (FR-3.1)? reqwest
0.13 has no per-phase timing API: its connector is internal, and `Response` exposes no
connection info beyond the remote address.
**Decision.** `wbl-http` reports the time to the response headers and the total. A breakdown
waits for Phase 4, where it would need our own `hyper-util` connector that wraps DNS, TCP and
rustls and times each step, and reqwest's pool and redirects rebuilt on top of it.
**Consequences.** The response panel shows "headers after X ms, total Y ms" until then. Phase 4
decides whether the breakdown is worth owning the connector.

## D-H11 HTTP errors: stable codes, messages without the URL (2026-11-18)

**Context.** reqwest's error text includes the full URL, which may carry an API key in its
query string, and errors are shown, logged by code and kept in history.
**Decision.** `wbl-http` classifies errors into `invalid_request`, `connect`, `tls`, `timeout`,
`redirect`, `body`, `cancelled` and `http`, and builds the message from the error's kind and its
causes, never its Display. TLS failures arrive as connection errors; they are recognised by
their causes (certificate, handshake). An HTTP error status is a response, not an error.
**Consequences.** A message can be less specific than reqwest's. The TLS check is a heuristic,
covered by a network test against a self-signed host (`--ignored`).
