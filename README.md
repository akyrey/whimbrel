# Whimbrel

A small desktop HTTP client in the spirit of Postman, without the account: requests are plain
TOML files in your git repository, secrets stay in the OS keychain, and OpenAPI documents (2.0
up to 3.2) import as collections.

**Status:** Phases 1–5 done (requests, collections, environments and secrets, responses and history, auth). See [docs/ROADMAP.md](docs/ROADMAP.md).

Sibling of [Dunlin](https://github.com/akyrey/dunlin); both are built on the
[Roost](https://github.com/akyrey/roost) kit.

## Build from source

Needs Rust (see `rust-toolchain.toml`), Node 22 and pnpm. To work on the Roost kit at the same
time, `scripts/link-roost.sh ../roost` points both builds at a local checkout
(`scripts/unlink-roost.sh` before committing).

```sh
pnpm --dir apps/desktop install
pnpm --dir apps/desktop tauri dev
```
