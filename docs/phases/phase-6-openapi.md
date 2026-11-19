# Phase 6: OpenAPI import and spec browser

Goal: any OpenAPI document (Swagger 2.0, OpenAPI 3.0, 3.1, 3.2), JSON or YAML, from a file or
a URL, imports as a collection whose requests are ready to send, and the spec itself can be
read in the app.

Design: [OPENAPI.md](../OPENAPI.md), [FILE_FORMAT.md](../FILE_FORMAT.md), D-H05, D-H14.

## Tasks

- [x] 6.1 `wbl-openapi` reading: JSON or YAML (`yaml-rust2`), the version from `swagger` /
      `openapi` (a newer 3.x is read as 3.2 with a warning), into untyped values (D-H14).
- [x] 6.2 `$ref` resolution: inside the document, to relative files, and to URLs only when the
      caller allows it; cycles are kept as links, never expanded.
- [x] 6.3 The normalized model (3.2 as the superset): info, servers with variables, tags with
      `parent` and `kind`, operations (including `QUERY` and `additionalOperations`), parameters
      (with `querystring`), request bodies and responses by media type (with `itemSchema`),
      examples (with `dataValue` / `serializedValue`), security schemes (with the device flow),
      webhooks. Upconversion from 2.0 (host, basePath, schemes, body and formData parameters,
      consumes / produces, securityDefinitions, definitions) and 3.0 (`nullable`, `example`).
- [x] 6.4 Example synthesis from JSON Schema 2020-12 (types and type arrays, `enum`, `const`,
      `default`, `examples`, formats, `allOf` / `oneOf` / `anyOf`, `prefixItems`), depth-limited.
- [x] 6.5 Request-level variables (`[variables]` in the request file), between the example and
      the environment in precedence (FR-4.4).
- [x] 6.6 Generation: a folder per tag (nested by `parent`), a request per operation with path
      parameters as `{{variables}}` (defaults in the request's variables), query and header
      parameters (optional ones off), the body and `Content-Type`, auth from security,
      environments per server (`baseUrl`, server variables, secret names for credentials), named
      examples as saved examples; `openapi/source.<ext>` and `openapi/import.toml` for re-import.
- [x] 6.7 Commands and UI: import from a file or a URL (asking before fetching remote refs) into
      a new or an open collection with a summary first; the spec browser (tags, operations,
      parameters, bodies and responses by media type, schema tree with links, examples) linked
      both ways with the generated requests.
- [x] 6.8 Fixtures for every row of OPENAPI.md's feature table, snapshot tests of the generated
      collection, and a 5 MB document imported in under 3 seconds.

## Outcome (2026-11-19)

- 26 `wbl-openapi` tests (90% lines): fixtures for 2.0, 3.0, 3.1, 3.2 and a multi-file spec,
  with insta snapshots of the generated collections. A generated 5.4 MB document (2900
  operations) is read, modelled, generated and written in 1.3 s in a debug build.
- The app: "OpenAPI…" in the sidebar (file or URL, a summary first, remote references fetched
  only after "Fetch them"), into a new collection or a new folder of an open one; "API specs"
  in the sidebar opens the browser; "Open request" and "Show in spec" link both ways.
- Not yet used from the feature table: callbacks and links (not shown in the browser),
  `$self` as the reference base, XML `nodeType`. Moved to Phase 8.
- Not checked: real large public specs (GitHub, Stripe, Kubernetes) and the real file dialog;
  these need the built app and the network (a manual check).

## Not in this phase

Re-import with a three-way diff and response validation (Phase 7).
