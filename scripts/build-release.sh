#!/bin/sh
# Build release archives: dist/chaude_<version>_<os>_<arch>.tar.gz + checksums.txt
# Usage: scripts/build-release.sh <version>   (env: MAIN_PKG=./cmd/chaude, TARGETS="darwin/arm64 ...")
set -eu
VERSION="${1:?usage: build-release.sh <version>}"
VERSION="${VERSION#v}"
MAIN_PKG="${MAIN_PKG:-./cmd/chaude}"
TARGETS="${TARGETS:-darwin/arm64}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
rm -rf dist && mkdir -p dist
for t in $TARGETS; do
  os="${t%/*}"; arch="${t#*/}"
  stage="dist/stage_${os}_${arch}"
  mkdir -p "$stage"
  CGO_ENABLED=0 GOOS="$os" GOARCH="$arch" go build -trimpath \
    -ldflags "-s -w -X main.version=${VERSION}" -o "$stage/chaude" "$MAIN_PKG"
  tar -C "$stage" -czf "dist/chaude_${VERSION}_${os}_${arch}.tar.gz" chaude
  rm -rf "$stage"
done
cd dist
if command -v sha256sum >/dev/null 2>&1; then sha256sum ./*.tar.gz; else shasum -a 256 ./*.tar.gz; fi | sed 's# \./# #' > checksums.txt
ls -l
