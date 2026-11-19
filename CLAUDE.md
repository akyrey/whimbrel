# Project instructions (Whimbrel)

Desktop HTTP client: Tauri 2 shell, Rust workspace backend, React + TypeScript frontend.
Collections are plain files in git; secrets live in the OS keychain. The design lives in `docs/`.
Code follows the docs; when they disagree, fix one of them in the same change.

## How to work on a phase

1. Read `docs/ROADMAP.md` to find the current phase, then its brief in `docs/phases/`.
2. Read the design docs the brief links to. Do not re-derive decisions recorded in `docs/DECISIONS.md`.
3. Before using a crate or npm package, check its current docs. Anything marked **Verify** is an
   assumption that has not been tested.
4. Work task by task, test first. Tick the task in the phase brief when its tests pass.
5. When a phase's acceptance criteria all pass, tick the phase in `docs/ROADMAP.md`.
6. If the implementation forces a design change, update the design doc and add a decision.

## Layout

```
crates/wbl-core              request, collection and environment model, interpolation (no I/O)
crates/wbl-http              sending requests: reqwest, cancellation, timings
apps/desktop/src-tauri       Tauri commands, in-flight request registry (thin)
apps/desktop/src             React frontend, organised by feature
```

Shared with Dunlin (D-H09): `roost-shell`, `roost-keychain` and `@roost/*` (ui, editor, grid,
palette, shortcuts, updates) come from the Roost repository by tag. To change them, work in a
Roost checkout and run `scripts/link-roost.sh ../roost`; run `scripts/unlink-roost.sh` before
committing (CI refuses a linked manifest). Code only Whimbrel uses stays here.

Dependency rule: `wbl-core` depends on no other workspace crate. Other `wbl-*` crates depend only
on `wbl-core` and Roost, except `wbl-openapi` and `wbl-interop`, which also use `wbl-format` (D-H16, D-H19). Only
`src-tauri` depends on everything.

## Rust conventions

- Errors: `thiserror` enums per crate with stable codes. No `unwrap`/`expect`/`panic!` outside
  tests. Never discard an error silently.
- No `unsafe`. Async runtime is tokio.
- Prefer returning new values over `&mut` parameters.
- Files 200-400 lines, 800 hard ceiling. Functions under 50 lines.
- Never log secrets, header values, URLs with credentials or query strings, or bodies.

## Frontend conventions

- TypeScript strict mode. No `any`.
- Types shared with Rust are generated with ts-rs, never hand-written.
- All backend calls go through `src/ipc/`; components never call `invoke` directly.
- State is immutable: Zustand stores replace objects, they do not mutate them.
- No `console.log` in committed code.

## Testing

TDD: failing test, minimal implementation, refactor. 80% line coverage per crate and for the
frontend. Tests that need the network beyond localhost are `#[ignore]`.

## Commits

Conventional commits. One task per commit where practical.
