#!/bin/sh
# Build release archives: dist/chaude_<version>_<os>_<arch>.tar.gz (windows: .zip) + checksums.txt
# Usage: scripts/build-release.sh <version>   (env: MAIN_PKG=./cmd/chaude, TARGETS="darwin/arm64 ...")
set -eu
VERSION="${1:?usage: build-release.sh <version>}"
VERSION="${VERSION#v}"
MAIN_PKG="${MAIN_PKG:-./cmd/chaude}"
TARGETS="${TARGETS:-darwin/arm64 linux/amd64 linux/arm64 windows/amd64}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MOD="$ROOT/deprecated/src/main/intt"   # Go module root; go commands run here
cd "$ROOT"
rm -rf dist && mkdir -p dist
for t in $TARGETS; do
  os="${t%/*}"; arch="${t#*/}"
  stage="dist/stage_${os}_${arch}"
  mkdir -p "$stage"
  bin=chaude; [ "$os" = windows ] && bin=chaude.exe
  (cd "$MOD" && CGO_ENABLED=0 GOOS="$os" GOARCH="$arch" go build -trimpath \
    -ldflags "-s -w -X main.version=${VERSION}" -o "$ROOT/$stage/$bin" "$MAIN_PKG")
  if [ "$os" = windows ]; then
    (cd "$stage" && zip -q -j "../chaude_${VERSION}_${os}_${arch}.zip" "$bin")
  else
    tar -C "$stage" -czf "dist/chaude_${VERSION}_${os}_${arch}.tar.gz" "$bin"
  fi
  rm -rf "$stage"
done
cd dist
files="$(ls chaude_*.tar.gz chaude_*.zip 2>/dev/null || true)"
if command -v sha256sum >/dev/null 2>&1; then sha256sum $files; else shasum -a 256 $files; fi > checksums.txt
ls -l
