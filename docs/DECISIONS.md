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
**Consequences.** (Breakdown done in D-H20.) The response panel shows "headers after X ms, total Y ms" until then. Phase 4
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

## D-H12 Variables resolve in the backend; the frontend never holds a secret value (2026-11-18)

**Context.** Previews, sending and the save guard all need secret values; the webview is the
part most exposed to what a response contains.
**Decision.** The frontend sends the request as written, with the collection, environment and
example; the backend lays the example over it, resolves variables (secrets from the
keychain), sends, and masks secrets in previews. Secret values travel only one way: from the
editor's password field to `secret_set`. Saving refuses text holding the value of any of the
collection's secrets. Example overlays are computed by `wbl-core` through two pure commands,
so the rules exist once.
**Consequences.** Variable autocomplete and hover (Phase 8) will need names and plain values
from the backend, secrets masked. A response that echoes a secret shows it: that is the
server's answer, not a store of ours.

## D-H13 Large bodies: 10 MB in memory, the rest on disk; long text shown raw and unwrapped (2026-11-19)

**Context.** FR-3.5 and Phase 4's acceptance: a 10 MB JSON body must show without freezing the
window. Pretty-printing, building a table and wrapping one long line all block the main thread.
**Decision.** The engine keeps up to 10 MB in memory; a longer body is written whole to the
app's cache (emptied at start) and its first 1 MB is shown, with "Save to file". The frontend
pretty-prints and offers the table view only up to 2 MB, and does not wrap text longer than
that: a compact 10 MB JSON body is one line, and wrapping it made the editor lay it out at once
(over a second); unwrapped, the longest task is under 100 ms.
**Consequences.** Bodies between 2 and 10 MB show raw, on one line when the server sent one.
Pretty-printing them would need the work off the main thread (a worker, or the backend).

## D-H14 OpenAPI: our own reader over untyped values; YAML with yaml-rust2 (2026-11-19)

**Context.** D-H05 left the parser open (spike `roas`). Real-world specs are often slightly
invalid (missing required fields, 3.0 keywords in 3.1 documents, `$ref` siblings), and a strict
typed model rejects documents other tools accept. `$ref` resolution across files and URLs, and
the upconversion from 2.0 and 3.0, need control over the raw document either way.
**Decision.** `wbl-openapi` reads documents into `serde_json::Value` (YAML through
`yaml-rust2`, the maintained YAML 1.2 parser, converted to JSON values) and builds the
normalized model (3.2 as the superset) leniently: missing optional parts are empty, unknown
keys are ignored, problems become warnings attached to the import, not failures. Schemas stay
JSON Schema values, with references rewritten to the model's own schema table so cycles are
links. No `roas` spike.
**Consequences.** We own the per-version mapping and test it with fixtures. Validation of the
spec itself is out of scope: the import reports what it could not understand.

## D-H15 What an import generates when the file format cannot say it (2026-11-19)

**Context.** A request has its query string in the URL (no disabled query parameters), and a
text body only. OpenAPI operations have optional query parameters, multipart bodies and
per-operation servers; a collection can hold several imported specs.
**Decision.** Required query parameters go in the URL; optional ones go in a saved example
"All parameters", so they are one click away and the base request stays minimal. Optional
headers and cookies are present but disabled. Bodies of media types that are not text
(multipart, binary) are not generated: the import says so. Operation-level `servers` are not
used (the environment's `baseUrl` applies). Each import keeps its spec and manifest in
`openapi/<slug of the title>/` (a second import of the same title gets `-2`), so a collection
can hold several specs. Environments that already exist by name are left alone.
**Consequences.** Multipart bodies need body kinds the format does not have yet (Phase 8 at the
earliest). Re-import (Phase 7) finds its manifest by the spec's directory.

## D-H16 `wbl-openapi` uses `wbl-format` (2026-11-19)

**Context.** The architecture had `wbl-openapi` emit `wbl-core` types and never write files.
But what an import makes is collection files: stored requests (with saved examples, auth and
request variables), environments, folders, plus the spec and its manifest, and the order and
naming rules for those live in `wbl-format`.
**Decision.** `wbl-openapi` depends on `wbl-format`. Generation stays pure (`generate` returns
stored requests and environments); `install` writes them through `wbl-format`'s collection
operations (one batch per folder, `create_requests`, so a 3000-operation spec is not quadratic).
`wbl-format` still depends only on `wbl-core`, so there is no cycle.
**Consequences.** Re-import (Phase 7) reads and rewrites the same files through the same crate.

## D-H17 Chaining: runtime variables, RFC 9535 JSONPath, re-import from the kept spec (2026-11-19)

**Context.** FR-6 asks for captures, assertions and runs without scripts; FR-7.5 asks for a
re-import that keeps the user's edits, which needs what was generated last time.
**Decision.** Captured values are *runtime variables*: per collection, in memory, between the
request's own variables and the environment (example → request → runtime → `.local` →
environment → collection). A capture marked secret is masked like any secret and, when the
active environment declares a secret of that name, saved to the keychain. JSONPath is RFC 9535
(`serde_json_path`), regexes are the `regex` crate (no look-around, linear time). Response
validation uses `jsonschema` (2020-12, the model's schema table as `$defs`). Re-import
regenerates the *old* collection from the spec kept in `openapi/<name>/source.*`, so the three
versions (old generated, new generated, current file) are compared field by field; the manifest
hash says whether a file was touched at all.
**Consequences.** Runtime variables are lost on restart (by design: tokens expire); secrets that
must survive go to the keychain. A change in the generator between versions shows up as a
change on re-import.

## D-H18 Form, multipart and file bodies; file paths relative to the collection (2026-11-19)

**Context.** FR-2.2 asks for form-urlencoded, multipart (fields and files) and binary file
bodies. Request files live in git and are shared, so a path to a file must mean the same thing
on every machine.
**Decision.** `Body` gains `Form { fields }`, `Multipart { fields }` (a field with `file = true`
holds a path) and `File { path }`. In the file: `[body] kind = "form" | "multipart"` with
`[[body.fields]]`, or `kind = "file"` with `path`. A relative path is taken from the
collection's folder when sending (so files can be kept beside the requests in git); an absolute
one is used as it is. The engine reads the files when sending; a missing file fails the send
naming the path. Multipart gets its `Content-Type` (with the boundary) from the engine, so an
import does not set one. This replaces D-H15's "multipart bodies are not generated".
**Consequences.** Files are read whole into memory before sending (no streaming upload yet).

## D-H19 Interop: Postman and `.http` through one import shape; secrets never copied (2026-11-19)

**Context.** People arrive with Postman collections and environments, or `.http` files; some
leave with `.http` files. Postman exports hold credentials typed into auth fields and
"secret" variables; scripts have no Whimbrel equivalent.
**Decision.** A crate `wbl-interop` (depending on `wbl-core` and `wbl-format`, like
`wbl-openapi`, D-H16) reads Postman v2.0/v2.1 collections and environments and `.http` files into
one shape (folders, stored requests, collection variables, environments), and an installer
writes it. A credential typed into Postman auth becomes a `{{variable}}` and a secret name in an
"Imported" environment, never a value; secret variables become secret names. Saved responses
whose request differs only in query, headers or body become saved examples. Scripts and
response handlers are reported, not run. `.http` export writes each request with the auth it
inherits as a header (with its variables); examples, captures and assertions have no `.http`
form and are left out.
**Consequences.** Users set imported secrets once in the keychain. Postman's dynamic variables
other than `$guid`, `$randomUUID`, `$timestamp`, `$isoTimestamp` stay as written, with a warning.

## D-H20 Timing breakdown: a timed resolver and connector layer, not our own connector (2026-11-20)

**Context.** D-H10 left DNS, connect and TLS times for later, expecting them to need our own
`hyper-util` connector. reqwest 0.13 has two hooks that avoid that: `dns_resolver` (a public
`Resolve` trait) and `connector_layer` (a tower layer around its whole connector, TLS included).
Clients are shared and cached, so neither hook knows which send it serves.
**Decision.** `wbl-http` installs a resolver that times `getaddrinfo` (as reqwest's default does
it, on a blocking thread) and a generic layer that times each connection. Each send runs inside
a tokio task-local recorder; hyper connects inside the send's own future, so the hooks find it
there. `Timing` gains `dnsMs` (`null` for an IP address or a reused connection) and `connectMs`
(TCP and TLS together, `null` for a reused connection); waiting and download are derived from
`headersMs` and `totalMs`. Both new fields default to `null`, so history saved before still reads.
**Consequences.** TCP and TLS are not separate: reqwest does both inside the layer. A connect
that hyper finishes in the background after the send took a pooled connection records nothing,
which is right, since that send did not wait for it. Supersedes the "breakdown waits" part of
D-H10.

## D-H21 Several named OAuth 2 tokens per configuration (2026-11-20)

**Context.** Phase 5 kept one token per collection and OAuth 2 configuration, replaced by each
sign-in. Testing as different users, or keeping a token while trying another scope, meant
signing in again each time, and a token could not be told apart from another.
**Decision.** A configuration keeps a list of named tokens, one of them in use. In the keychain
slot `oauth`, the entry `<cacheKey>` holds an index (ids, names, when obtained, the id in use)
and each token has its own entry `<cacheKey>:<id>`, so no entry grows with the list (Windows
Credential Manager holds about 2.5 kB per entry). A sign-in adds a token, named by the user or
`Token N`, and uses it; a refresh replaces the token in use and keeps its name. Removing the
token in use leaves none in use rather than picking another. A single token stored by an
earlier version becomes the first of the list when read. Token values still never reach the
frontend (D-H12): it sees names, times, scope and whether a token can be refreshed.
**Consequences.** `oauth_forget` is replaced by `oauth_token_change` (rename, select, remove).
Tokens are listed per configuration, not across the collection, since the keychain cannot be
enumerated.

## D-H22 Headers inherited from folders and the collection (2026-11-20)

**Context.** Large collections repeat the same headers (tenant, client version, `Accept`) on
every request. FILE_FORMAT.md already reserved `_folder.toml` for "auth/headers/variables
inherited by children", but only auth was built.
**Decision.** `collection.toml` and `_folder.toml` take `[[headers]]` in the request's format.
A request sends the collection's headers first, then each folder's from the outside in, then
its own. A header at a deeper level replaces every inherited header of the same name, ignoring
case; a disabled one turns the inherited header off without sending anything. Replacing by
name (rather than the example overlay's in-place update) keeps a request's duplicate headers.
Inheritance applies before the example and variables, so inherited values use the request's
variables. A `_folder.toml` that cannot be read fails the send with its error instead of
dropping its headers unnoticed (unlike auth, which treats it as inheriting). `.http` export
writes each request with the headers it inherits. Saving refuses literal secret values, like
auth. The request's Headers tab lists what it inherits, struck through where replaced.
**Consequences.** Older versions keep the `[[headers]]` of these files on rewrite (unknown keys
are preserved) but do not send them. Folder variables (FR-4.4) are still to do.

## D-H23 An MCP server in the app binary, on a send pipeline shared with the app (2026-11-20)

**Context.** AI tools should manage collections and environments as Postman's MCP server lets
them (FR-10). The files can already be edited by hand, but only `wbl-format` writes them in
canonical form, and sending needs scopes, secrets and auth, which today live in `src-tauri`
(`resolve.rs`, `secrets.rs`, `auth_apply.rs`, the runner). Postman's server talks to its cloud
API; ours has none to talk to.
**Decision.** Two new crates. `wbl-engine` takes the send pipeline out of `src-tauri` without
Tauri types: scopes and resolution, the keychain wrapper and secret guard, auth context, checks,
the folder runner, and reading specs and Postman/`.http` files for an import; `src-tauri`
keeps its commands as thin callers. `wbl-mcp` is the server,
built on the official Rust SDK `rmcp` over stdio, with tools over `wbl-format`, `wbl-engine`,
`wbl-openapi` and `wbl-interop`. It ships inside the app binary: `whimbrel mcp [--collection
<dir>]…` starts the server instead of the window, so there is one signed binary, updated with
the app, and no sidecar. The server works on files directly; the running app sees its writes
through the watcher like any outside change (a dirty tab gets the conflict prompt). Rules:
- Only collections passed with `--collection` are reachable, by folder or name; paths go
  through `wbl-format`'s path resolution, so nothing escapes a collection (`not_reachable`,
  `invalid_path`).
- Secret values never leave the server (D-H04, D-H12). Variables are listed as names with
  `secret: true`; a tool may mark a variable secret but never set its value. Saving refuses literal secret values, as the app does.
  Sent requests are returned masked, and every known secret value (those the request used, the
  environment's, and what a secret capture just took) is masked in the response, the capture
  values and the assertion messages, as history masks them (`wbl-engine::deliver`).
- Sending with a protected environment is refused (`protected_environment`); the user sends
  with it from the app, where the confirmation lives.
- OAuth 2 uses the cached token and refreshes it; without one the send fails with
  `oauth_required`, as in the app (sign in from the app).
- Response bodies returned to the tool are capped (text, 256 kB, truncation reported).
- MCP sends are not recorded in history in this phase.
**Consequences.** The dependency rule gains two exceptions: `wbl-engine` depends on `wbl-core`,
`wbl-format`, `wbl-http`, `wbl-auth`, `wbl-openapi`, `wbl-interop` and Roost; `wbl-mcp` on the `wbl-*` crates it serves;
`src-tauri` on both. Logging must never write to stdout in MCP mode, since stdout is
the protocol channel. Errors are tool results with `isError` and `{code, message, detail?}`,
so the model reads them. **Verified** with `rmcp` 3.11: MCP 2026-07-28 forbids server-to-client
requests during a call (elicitation becomes a multi-round-trip `InputRequiredResult`) and
deprecates roots (SEP-2577), so asking the user mid-call or discovering collections from the
client depends on the protocol version negotiated. Refusing protected environments and taking
collections from arguments works with every version and keeps a human confirmation where it
already is. **Verify**: that a binary inside the macOS `.app` bundle runs headless from a
terminal.

