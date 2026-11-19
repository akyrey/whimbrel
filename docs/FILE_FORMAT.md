# Collection file format (v1, draft)

Goals: readable without the app, one request per file so merges are local, deterministic output
so saving an unchanged request is a no-op, and no secret values anywhere.

## Layout

```
my-api/
  collection.toml            id, name, format version, default auth, collection variables
  .gitignore                 contains ".local/" (added by the app)
  environments/
    local.toml
    staging.toml
    production.toml          protected = true
  .local/                    git-ignored per-user overrides (never committed)
    staging.toml
  users/
    _folder.toml             folder name, order, auth/headers/variables inherited by children
    list-users.req.toml
    create-user.req.toml
  openapi/
    my-api/                  one per imported spec (D-H15)
      source.yaml            the imported spec, kept for re-import diffing
      import.toml            title, source location, spec hash, operationId → request file map
```

## collection.toml

```toml
format = 1
id = "4f6c1d1e-…"          # stable; keys keychain entries. Never regenerate.
name = "My API"

[variables]
apiVersion = "v2"

[auth]
type = "bearer"
token = "{{accessToken}}"
```

## A request

```toml
format = 1
name = "Create user"
order = 20
method = "POST"
url = "{{baseUrl}}/users"

[[query]]
name = "notify"
value = "true"
enabled = false               # written only when false

[[headers]]
name = "Content-Type"
value = "application/json"

[body]
kind = "json"
text = """
{
  "name": "user-{{$randomInt}}",
  "email": "test@example.com"
}
"""

[auth]
type = "inherit"

[[captures]]                 # FR-6.1: into a runtime variable (D-H17)
var = "userId"
from = "body"                # or "header" (path is then the header name)
path = "$.id"                # JSONPath, RFC 9535; "" is the whole body
# regex = "(\\d+)"          # optional: first group, else the whole match
# secret = true              # masked; saved to the keychain if the environment declares it

[[assertions]]               # FR-6.2
check = "status"
equals = 201

[[assertions]]
check = "json"               # also "header" (name, equals?) and "time" (below_ms)
path = "$.email"
equals = "test@example.com"  # left out: the value only has to exist

[openapi]                  # present only on generated requests
operationId = "createUser"
# removed = true           # set by a re-import when the operation left the spec

# Saved examples (FR-2.8, D-H08): overlays on the request above.
[[examples]]
id = "b1e0…"              # stable; the tree, tabs and runner steps reference examples by id
name = "Example A"
order = 10

[examples.variables]       # request-scope variables, highest precedence
userRole = "admin"

[[examples.query]]         # matched by name: replaces the value, or adds the param
name = "notify"
value = "true"
enabled = true

[[examples.headers]]
name = "X-Tenant"
value = "{{tenantId}}"    # references are fine; secret values are refused on save

[examples.body]            # if present, replaces the whole body
kind = "json"
text = """
{ "name": "Ada", "email": "ada@example.com", "role": "{{userRole}}" }
"""

[examples.response]        # optional saved reference response (FR-2.8.4)
status = 201
headers = [{ name = "Content-Type", value = "application/json" }]
file = "create-user.examples/example-a.response.json"
```

Overlay rules: fields an example does not mention come from the base request. An example's
`[examples.body]` with `kind = "none"` removes the request's body. To remove a
query parameter or header, an example sets `enabled = false` on it. A body or response
over 64 KB goes to a sidecar file in `<request>.examples/`.

Lists are arrays of tables, not maps, so order and duplicate headers survive. Large bodies may
live in a sidecar file: `[body] kind = "json" file = "create-user.body.json"`.

Request-level `[variables]` (FR-4.4) sit between the example's and the environment's:

```toml
[variables]
userId = "42"               # imported path parameters default here
```

## An environment

```toml
format = 1
name = "production"
protected = true

[variables]
baseUrl = "https://api.example.com"

[secrets]
# names only; values are in the OS keychain under (collection id, "production", name)
apiKey = { description = "From 1Password: Example API prod" }
```

## Rules

- Multi-line text is written as a TOML `"""` string (what `toml_edit` produces; quotes and
  backslashes are escaped only where TOML needs it).

- Unknown keys are preserved on rewrite (`toml_edit`), so newer versions do not lose data in
  older ones and hand-added comments survive.
- Variable precedence with an active example: example variables > request > folder > …
  (extends FR-4.4).
- `format` is bumped only for breaking changes; `wbl-format` migrates forward on read and
  writes the newest format only after the user saves.
- File names are slugs of the request name, made unique within the folder; renaming a request
  renames its file. Order comes from `order` (gaps of 10 so a move rewrites one file).
- Interop (later phase): import/export `.http` files (JetBrains/VS Code REST Client) and
  Postman v2.1 collections. Postman secrets become secret names, never values.
