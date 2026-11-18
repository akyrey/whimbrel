#!/bin/sh
# Undoes scripts/link-roost.sh: back to the Roost tag in the committed manifests and lockfiles.
set -eu
cd "$(dirname "$0")/.."
git checkout -- Cargo.toml apps/desktop/package.json
for lock in Cargo.lock apps/desktop/pnpm-lock.yaml; do
  if git ls-files --error-unmatch "$lock" >/dev/null 2>&1; then
    git checkout -- "$lock"
  else
    # No committed lockfile yet (Roost not published when it was written): drop the linked one.
    rm -f "$lock"
  fi
done
if [ -f apps/desktop/pnpm-lock.yaml ]; then
  pnpm --dir apps/desktop install --frozen-lockfile
fi
echo "Unlinked: using the Roost tag again."
