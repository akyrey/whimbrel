# Whimbrel

A small desktop HTTP client in the spirit of Postman, without the account: requests are plain
TOML files in your git repository, secrets stay in the OS keychain, and OpenAPI documents (2.0
up to 3.2) import as collections.

**Status:** see [docs/ROADMAP.md](docs/ROADMAP.md).

Sibling of [Dunlin](https://github.com/akyrey/dunlin); both are built on the
[Roost](https://github.com/akyrey/roost) kit.

## Use with AI tools (MCP)

The app binary is also a [Model Context Protocol](https://modelcontextprotocol.io) server, so AI
tools such as Claude Desktop, Claude Code or Cursor can read and change your collections and
send requests:

```sh
<whimbrel> mcp --collection ~/code/shop/api --collection ~/code/billing/api
```

where `<whimbrel>` is the app's executable: on macOS
`/Applications/Whimbrel.app/Contents/MacOS/Whimbrel`, on Linux the `whimbrel` command the package
installs (or the AppImage). Only the collections named with `--collection` are reachable.

Claude Desktop (`claude_desktop_config.json`), Cursor (`.cursor/mcp.json`) and other clients that
use the same format:

```json
{
  "mcpServers": {
    "whimbrel": {
      "command": "/Applications/Whimbrel.app/Contents/MacOS/Whimbrel",
      "args": ["mcp", "--collection", "/Users/me/code/shop/api"]
    }
  }
}
```

Claude Code:

```sh
claude mcp add whimbrel -- /Applications/Whimbrel.app/Contents/MacOS/Whimbrel mcp --collection "$PWD/api"
```

Tools: `list_collections`, `get_tree`, `get_request`, `list_environments`, `get_environment`;
`create_request`, `update_request`, `create_folder`, `rename`, `move`, `reorder`, `duplicate`,
`delete`, `set_headers`, `set_auth`; `create_environment`, `set_variables`,
`set_collection_variables`, `delete_environment`; `import_openapi`, `import_file` (Postman,
`.http`), `import_curl`, `export_http`; `send_request`, `run_folder`.

What the server never does:

- **Show or set a secret value.** Secrets appear by name only; set their values in the app.
  Known secret values are masked in requests, responses and check results, and literal secret
  values are refused when saving.
- **Send with a protected environment.** Send with it from the app, which asks first.
- **Sign in.** OAuth 2 requests use the token you got in the app (refreshed when needed).
- **Record history.** Sends from AI tools are not added to the app's history.

Changes are plain file writes, so the open app reloads them like any other change on disk, and
git shows what the tool did.

## Build from source

Needs Rust (see `rust-toolchain.toml`), Node 22 and pnpm. To work on the Roost kit at the same
time, `scripts/link-roost.sh ../roost` points both builds at a local checkout
(`scripts/unlink-roost.sh` before committing).

```sh
pnpm --dir apps/desktop install
pnpm --dir apps/desktop tauri dev
```
