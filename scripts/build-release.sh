#!/bin/sh
# Build release archives: dist/chaude_<version>_<os>_<arch>.tar.gz (.zip for windows) + checksums.txt
# Usage: scripts/build-release.sh <version>   (env: MAIN_PKG=./cmd/chaude, TARGETS="darwin/arm64 windows/amd64 ...")
set -eu
VERSION="${1:?usage: build-release.sh <version>}"
VERSION="${VERSION#v}"
MAIN_PKG="${MAIN_PKG:-./cmd/chaude}"
TARGETS="${TARGETS:-darwin/arm64 darwin/amd64 linux/amd64 linux/arm64 windows/amd64 windows/arm64}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
rm -rf dist && mkdir -p dist
for t in $TARGETS; do
  os="${t%/*}"; arch="${t#*/}"
  stage="dist/stage_${os}_${arch}"
  mkdir -p "$stage"
  bin=chaude
  [ "$os" = windows ] && bin=chaude.exe
  CGO_ENABLED=0 GOOS="$os" GOARCH="$arch" go build -trimpath \
    -ldflags "-s -w -X main.version=${VERSION}" -o "$stage/$bin" "$MAIN_PKG"
  if [ "$os" = windows ]; then
    if command -v zip >/dev/null 2>&1; then
      (cd "$stage" && zip -q "../chaude_${VERSION}_${os}_${arch}.zip" "$bin")
    else
      (cd "$stage" && python3 -m zipfile -c "../chaude_${VERSION}_${os}_${arch}.zip" "$bin")
    fi
  else
    tar -C "$stage" -czf "dist/chaude_${VERSION}_${os}_${arch}.tar.gz" "$bin"
  fi
  rm -rf "$stage"
done
cd dist
if command -v sha256sum >/dev/null 2>&1; then sha256sum ./*.tar.gz ./*.zip; else shasum -a 256 ./*.tar.gz ./*.zip; fi | sed 's# \./# #' > checksums.txt
ls -l
