# Phase 3: Environments, variables, secrets, saved examples

Goal: `{{variables}}` resolve from the active environment; secrets live in the OS keychain and
never in a file, a log or a preview; requests have saved examples.

Design: [PRD.md](../PRD.md) FR-2.4, FR-2.5, FR-2.8, FR-4; [FILE_FORMAT.md](../FILE_FORMAT.md);
D-H04, D-H08.

## Tasks

- [x] 3.1 `wbl-core` interpolation: `{{name}}` and the dynamic values `{{$uuid}}`,
      `{{$timestamp}}`, `{{$isoTimestamp}}`, `{{$randomInt}}` in the URL, header names and values
      and the body. Scopes in precedence order: example, `.local` override, environment,
      collection. Unknown names and secrets without a value are reported together, before
      anything is sent. The result lists the secret values it used, for masking.
- [x] 3.2 Saved examples in `wbl-core`: an overlay (variables, query and header values by
      name, a replacement body) applied to a request without changing it.
- [x] 3.3 `wbl-format`: environment files (`environments/<name>.toml`: variables, secret names,
      `protected`), `.local/<name>.toml` overrides, `[variables]` in `collection.toml`,
      `[[examples]]` in request files; all with the write rules of Phase 2.
- [ ] 3.4 Secrets in the keychain (`roost-keychain`, service `whimbrel`, account
      `<collection id>:<environment>:<name>`): set, clear, and which are set. Values never cross
      to the frontend.
- [ ] 3.5 Sending resolves in the backend with the chosen environment and example; a preview
      command returns the resolved request with secret values masked. Saving a request or an
      example whose text holds the value of one of the collection's secrets is refused.
- [ ] 3.6 Frontend: environment picker per collection, environment editor (variables, secret
      names with set / clear), the resolved preview, unknown and unset names shown before
      sending, a confirmation before sending with a protected environment.
- [ ] 3.7 Frontend: examples (pick, save the current edits as an example, update, delete,
      send with one), listed under their request in the tree and in the palette.
- [ ] 3.8 Sentinel test: a secret set, used, sent, previewed and refused on save never appears
      in any file under the collection or the app's log folder.

## Not in this phase

Variable autocomplete and hover in the editors (needs the URL field to become an editor;
Phase 8), request- and folder-level variables, captures (Phase 7).
