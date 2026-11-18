#!/bin/sh
# Undoes scripts/link-roost.sh: back to the Roost tag in the committed manifests and lockfiles.
set -eu
cd "$(dirname "$0")/.."
git checkout -- Cargo.toml Cargo.lock apps/desktop/package.json apps/desktop/pnpm-lock.yaml
pnpm --dir apps/desktop install --frozen-lockfile
echo "Unlinked: using the Roost tag again."
