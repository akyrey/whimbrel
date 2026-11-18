#!/bin/sh
# Points Whimbrel at a local Roost checkout (default ../roost) instead of the tag in Cargo.toml
# and apps/desktop/package.json, for working on both at once. While linked those two files and
# their lockfiles are changed: do not commit them (CI refuses a linked manifest). Undo with
# scripts/unlink-roost.sh.
set -eu
cd "$(dirname "$0")/.."
roost=$(cd "${1:-../roost}" && pwd)
[ -f "$roost/crates/roost-shell/Cargo.toml" ] || { echo "No Roost checkout at $roost" >&2; exit 1; }

sed -i.bak -E "s#^(roost-[a-z]+) = \{ git = \"https://github.com/akyrey/roost\", tag = \"[^\"]+\" \}#\1 = { path = \"$roost/crates/\1\" }#" Cargo.toml
sed -i.bak -E "s#\"github:akyrey/roost\#[^&]+&path:/packages/([a-z]+)\"#\"link:$roost/packages/\1\"#" apps/desktop/package.json
rm -f Cargo.toml.bak apps/desktop/package.json.bak

pnpm --dir apps/desktop install
cargo metadata --format-version 1 >/dev/null
echo "Linked to $roost. Run scripts/unlink-roost.sh before committing."
