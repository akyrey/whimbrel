# Changelog

All notable changes to Whimbrel. The format follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

- Requests as TOML files in git: collections, folders, saved examples, request variables.
- Environments with secrets in the OS keychain; `.local` overrides; protected environments.
- Responses: pretty, raw, table, hex and image views; streams (SSE, JSON Lines); large bodies.
- Auth: basic, bearer, API key, OAuth 2 (client credentials, authorization code with PKCE,
  device code), per-collection TLS.
- OpenAPI 2.0 to 3.2 import, a spec browser, re-import with a three-way merge, response
  validation.
- Captures, assertions, runtime variables and a folder runner.
- Form, multipart and file bodies; a query table; variable completion.
- Postman v2.1 and `.http` import, `.http` export.
- Settings, palette commands for every action, tabs restored with unsaved edits.
- Signed in-app updates from GitHub releases.
