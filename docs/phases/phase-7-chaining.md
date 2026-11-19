# Phase 7: Chaining, assertions, runner, re-import

Goal: requests pass values to each other without scripts, check their responses, and run as a
folder; an imported spec can be re-imported without losing the user's edits, and responses can
be checked against the spec.

Design: [PRD.md](../PRD.md) FR-6, FR-7.5, FR-7.6; [FILE_FORMAT.md](../FILE_FORMAT.md);
[OPENAPI.md](../OPENAPI.md) "Re-import"; D-H17.

## Tasks

- [x] 7.1 `wbl-core`: captures (from the body by JSONPath, RFC 9535, or from a header, with an
      optional regex) and assertions (status equals, header exists/equals, JSONPath
      exists/equals, time below), evaluated against what was received. `[[captures]]` and
      `[[assertions]]` in request files.
- [x] 7.2 Runtime variables: captures set them per collection, after the request's own variables
      and before the environment; secret captures are masked, and saved to the keychain when
      the active environment declares that secret. Sending returns the checks with the response;
      the request view edits captures and assertions and shows pass/fail.
- [x] 7.3 The runner: a folder's requests in tree order, each as it is, with one saved example,
      or once per example; stop on the first failure or go on; results streamed into a grid;
      cancellable.
- [ ] 7.4 Response validation (FR-7.6): a response checked against its operation's schema
      (status, then `2XX`, then `default`; media type), errors with their JSON pointer.
- [ ] 7.5 Re-import (FR-7.5): added operations created, removed ones marked, changed ones
      regenerated when untouched since the import, and merged field by field when edited, with
      a choice per conflicting field.
- [ ] 7.6 Acceptance: login → capture token → authorised call passes as a run (E2E and a Rust
      test against a local server); re-importing a changed spec keeps user edits.
