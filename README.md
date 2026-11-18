# Whimbrel

A small desktop HTTP client in the spirit of Postman, without the account: requests are plain
TOML files in your git repository, secrets stay in the OS keychain, and OpenAPI documents (2.0
up to 3.2) import as collections.

**Status:** Phase 1 (skeleton). See [docs/ROADMAP.md](docs/ROADMAP.md).

Sibling of [Dunlin](https://github.com/akyrey/dunlin); both are built on the
[Roost](https://github.com/akyrey/roost) kit.

## Build from source

Needs Rust (see `rust-toolchain.toml`), Node 22 and pnpm. Until Roost is published, link a local
checkout first: `scripts/link-roost.sh ../roost`.

```sh
pnpm --dir apps/desktop install
pnpm --dir apps/desktop tauri dev
```
