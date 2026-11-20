#!/usr/bin/env bash
# Prepares a release: writes its CHANGELOG.md section and sets the version. See docs/RELEASING.md.
#
#   scripts/release.sh X.Y.Z
#
# Notes written by hand under `## [Unreleased]` become the release's section. With none, the
# section is drafted by git-cliff (cliff.toml) from the feat, perf and fix commits since the
# last tag. Then it sets X.Y.Z in Cargo.toml (and Cargo.lock) and apps/desktop/package.json.
# It does not commit or tag: review CHANGELOG.md first.
set -euo pipefail

cd "$(dirname "$0")/.."
version="${1:-}"
changelog=CHANGELOG.md
unreleased='## [Unreleased]'

fail() { echo "release.sh: $*" >&2; exit 1; }

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "usage: scripts/release.sh X.Y.Z"
git rev-parse -q --verify "refs/tags/v$version" >/dev/null && fail "tag v$version exists"
grep -qF "## [$version]" "$changelog" && fail "$changelog already has a section for $version"
grep -qxF "$unreleased" "$changelog" || fail "$changelog has no '$unreleased' heading"
[ -z "$(git status --porcelain)" ] || fail "the working tree has changes; commit or stash them"

# The lines between [Unreleased] and the next release heading.
hand_written=$(awk -v h="$unreleased" '$0 == h {on=1; next} /^## \[/ {on=0} on' "$changelog" |
  sed '/./,$!d')
today=$(date +%Y-%m-%d)
section=$(mktemp)
log=$(mktemp)
next=$(mktemp)
trap 'rm -f "$section" "$log" "$next"' EXIT

if [ -n "$hand_written" ]; then
  printf '## [%s] - %s\n\n%s\n' "$version" "$today" "$hand_written" >"$section"
  echo "Using the notes under [Unreleased]."
else
  command -v git-cliff >/dev/null || fail "git-cliff is not installed (brew install git-cliff)"
  git-cliff --config cliff.toml --unreleased --tag "v$version" --strip all >"$section" 2>"$log" ||
    { cat "$log" >&2; fail "git-cliff failed"; }
  grep -q '^- ' "$section" ||
    fail "no feat, perf or fix commits since the last tag; write notes under [Unreleased]"
  echo "Drafted the notes from the commits since the last tag."
fi

# [Unreleased] stays, empty, above the new section; its old lines are dropped.
awk -v h="$unreleased" -v file="$section" '
  $0 == h { print; print ""; while ((getline line < file) > 0) print line; print ""; skip=1; next }
  skip && /^## \[/ { skip=0 }
  !skip
' "$changelog" >"$next"
mv "$next" "$changelog"

sed -i.bak -E "/^\[workspace\.package\]/,/^\[/ s/^version = \"[^\"]+\"/version = \"$version\"/" Cargo.toml
sed -i.bak -E "s/^  \"version\": \"[^\"]+\",/  \"version\": \"$version\",/" apps/desktop/package.json
rm -f Cargo.toml.bak apps/desktop/package.json.bak
cargo update --workspace --quiet

cat <<EOF

Release $version prepared. Review CHANGELOG.md, then:

  git commit -am "chore: release $version"
  git tag v$version
  git push origin main v$version
EOF
