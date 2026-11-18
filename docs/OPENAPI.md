# OpenAPI support

Target: every published version up to the latest, **3.2.0 (Sept 2025)**: Swagger 2.0, OpenAPI
3.0.0–3.0.4, 3.1.0–3.1.2, 3.2.0. Unknown future 3.x minor versions are parsed as the newest
known version with a warning.

## Pipeline

```
bytes ─► detect (JSON/YAML, `swagger`/`openapi` field)
      ─► parse into the version's model
      ─► resolve $refs (internal, relative files, URLs with confirmation; cycles kept as links)
      ─► normalise to one internal model (wbl-openapi::Api) — 3.2 is the superset
      ─► (a) generate wbl-core collection   (b) feed the spec browser   (c) diff for re-import
```

One normalised model means generation, browsing and diffing are written once. Upconversion
rules: Swagger 2.0 `host`/`basePath`/`schemes` → servers, `consumes`/`produces` → media types,
body/formData parameters → request bodies, `securityDefinitions` → security schemes. 3.0 →
3.1: `nullable` → type arrays, `example` → `examples`.

## Version features to handle explicitly

| Version | Feature | Effect in the app |
|---|---|---|
| 2.0 | `host`/`basePath`, formData, `collectionFormat` | servers; multipart/urlencoded bodies; array serialisation |
| 3.0 | servers with variables, `style`/`explode`, callbacks, links | server variables → env variables; correct param serialisation; links shown in browser (and offered as captures) |
| 3.1 | JSON Schema 2020-12 (`type` arrays, `const`, `$defs`, `prefixItems`, `$dynamicRef`), `webhooks`, `pathItems` in components, `$ref` siblings | schema renderer and example synthesiser understand 2020-12; webhooks shown read-only |
| 3.2 | `QUERY` method, `additionalOperations` (custom methods) | requests with those methods |
| 3.2 | hierarchical tags (`parent`, `kind`, `summary`) | nested folders; `kind` filters (e.g. hide `badge` tags) |
| 3.2 | sequential/streaming media (`itemSchema`, SSE, JSON Lines) | streaming response viewer, per-item schema |
| 3.2 | `in: querystring` parameter | whole query string from one schema |
| 3.2 | OAuth 2 device authorization flow, `oauth2MetadataUrl`, `deprecated` schemes | FR-5.2 device flow; metadata discovery |
| 3.2 | example `dataValue` / `serializedValue`, `$self`, response `summary`, XML `nodeType` | used in examples, ref base URI, browser labels |

Each row gets a fixture in `crates/wbl-openapi/tests/fixtures/` (prefer the OAI's own examples and
well-known public specs: Petstore variants, GitHub, Stripe, Kubernetes) and a snapshot test of
the generated collection (`insta`, as in Dunlin).

## Generation rules

- Folder per tag (first tag; untagged → path-segment folder). 3.2 tag `parent` → nested folders.
- Request per operation, file named from `operationId` or `method-path`; `[openapi] operationId`
  links it back.
- Path params → `{{paramName}}` with a folder- or request-level variable defaulting to the
  example; required query/header params enabled, optional ones present but disabled.
- Body: first of `examples`, `example`, or synthesised from the schema (respecting `required`,
  `enum`, `format`, `default`, depth-limited on cycles).
- Named `examples` on the request body (and on parameters) become saved examples (D-H08), one
  per name; parameter examples with matching names merge into one saved example. Response
  examples with the same name become that example's reference response.
- Servers → `baseUrl` in one environment per server (named from `description`), server
  variables → environment variables.
- Security → collection/operation auth; credentials become secret names in each environment.

## Re-import

`openapi/import.toml` stores the spec hash and, per operation, a hash of what was generated.
On re-import: operation removed → mark request (do not delete); added → create; changed and the
request is untouched since generation (hash matches) → regenerate; changed and edited by the
user → show a three-way diff (old generated / new generated / current) to accept per field.

## Spec browser

A read-only view built on the normalised model: tag tree, operation page (parameters table,
request body and responses with media-type tabs, schema tree with expand/collapse and `$ref`
links, examples in the code editor), and "Open request" / "Show in spec" both ways. Large specs
use the grid's virtualisation for long lists.

## Library choice

See D-H05. `roas` claims 2.0/3.0/3.1/3.2 models; `oas3` covers 3.1 only. Plan: spike `roas` in
Phase 6 against the fixture set; if it fails on real specs, own the model with serde over
`serde_json::Value` (the normalised model is ours either way).
