#!/bin/sh
# Bump the release version in place (no commit, no tag).
# Usage: scripts/bump-version.sh <X.Y.Z>
# Updates: src/main/intt/cmd/chaude/main.go (version var), CHAUDE_VERSION=vX.Y.Z in README.md + README.ko.md,
# **버전 X.Y.Z** in docs/claude-lean/README.md, and CHANGELOG.md ("## Unreleased" -> "## X.Y.Z - DATE" + fresh "## Unreleased").
set -eu
VERSION="${1:?usage: bump-version.sh <X.Y.Z>}"
case "$VERSION" in
  v*) echo "bump-version: pass X.Y.Z without the v prefix" >&2; exit 1 ;;
esac
if ! printf '%s\n' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "bump-version: not semver X.Y.Z: $VERSION" >&2; exit 1
fi
DATE="$(date +%Y-%m-%d)"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

grep -q '^## Unreleased$' CHANGELOG.md || { echo "bump-version: CHANGELOG.md has no '## Unreleased' line" >&2; exit 1; }

# sed -i.bak works on both GNU and BSD sed; the backups are removed right away.
sed -i.bak "s/^var version = \"[^\"]*\"/var version = \"$VERSION\"/" src/main/intt/cmd/chaude/main.go
sed -i.bak "s/CHAUDE_VERSION=v[0-9][0-9.]*/CHAUDE_VERSION=v$VERSION/g" README.md README.ko.md
sed -i.bak "s/\*\*버전 [0-9][0-9.]*\*\*/**버전 $VERSION**/" docs/claude-lean/README.md
rm -f src/main/intt/cmd/chaude/main.go.bak README.md.bak README.ko.md.bak docs/claude-lean/README.md.bak

# Turn the Unreleased section into the new version and open a fresh empty Unreleased above it.
tmp="$(mktemp)"
awk -v v="$VERSION" -v d="$DATE" '
  $0 == "## Unreleased" && !done { print "## Unreleased"; print ""; print "## " v " - " d; done = 1; next }
  { print }
' CHANGELOG.md > "$tmp"
cat "$tmp" > CHANGELOG.md   # keep file mode
rm -f "$tmp"

# Verify every target now carries the new version.
grep -q "^var version = \"$VERSION\"\$" src/main/intt/cmd/chaude/main.go
grep -q "CHAUDE_VERSION=v$VERSION bash" README.md
grep -q "CHAUDE_VERSION=v$VERSION bash" README.ko.md
grep -q "\*\*버전 $VERSION\*\*" docs/claude-lean/README.md
grep -q "^## $VERSION - $DATE\$" CHANGELOG.md
echo "bump-version: $VERSION ($DATE) applied"
