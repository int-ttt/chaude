#!/usr/bin/env bash
# mac + linux: make sure Go is available (installs a project-local copy under .tools/go if missing or too old),
# check python3 (Claude Code hooks are .claude/hooks/*.py), build bin/chaude, write .claude/settings.json for this OS.
# Usage: platform_setup/install.sh [--cross]     --cross also builds all OS/arch binaries into dist/
# Env:   GO_VERSION (default below) = version downloaded when no usable Go is found
set -euo pipefail
GO_VERSION="${GO_VERSION:-1.27.3}"
GO_MIN_MINOR=22   # go.mod: go 1.22
root="$(cd "$(dirname "$0")/.." && pwd)"
cross=0
for a in "$@"; do
  case "$a" in --cross) cross=1;; -h|--help) sed -n 2,5p "$0"; exit 0;; *) echo "unknown option: $a" >&2; exit 2;; esac
done

case "$(uname -s)" in Darwin) os=mac; goos=darwin;; Linux) os=linux; goos=linux;; *) echo "unsupported: $(uname -s)" >&2; exit 1;; esac
case "$(uname -m)" in x86_64|amd64) goarch=amd64;; arm64|aarch64) goarch=arm64;; *) echo "unsupported arch: $(uname -m)" >&2; exit 1;; esac

go_ok() {  # $1 = go binary; true if version >= 1.$GO_MIN_MINOR
  local v minor
  v="$("$1" env GOVERSION 2>/dev/null)" || return 1          # e.g. go1.23.5
  minor="$(printf '%s' "$v" | sed -nE 's/^go1\.([0-9]+).*/\1/p')"
  [ -n "$minor" ] && [ "$minor" -ge "$GO_MIN_MINOR" ]
}

sha256() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }

fetch() {  # $1 url, $2 out file
  if command -v curl >/dev/null; then curl -fsSL "$1" -o "$2"
  elif command -v wget >/dev/null; then wget -q "$1" -O "$2"
  else echo "need curl or wget to download Go" >&2; exit 1; fi
}

install_go() {
  local pkg="go${GO_VERSION}.${goos}-${goarch}.tar.gz" tmp
  tmp="$(mktemp -d)"
  echo "Go not found (or older than 1.${GO_MIN_MINOR}): downloading ${pkg} into .tools/go" >&2
  fetch "https://dl.google.com/go/${pkg}" "$tmp/$pkg"
  fetch "https://dl.google.com/go/${pkg}.sha256" "$tmp/sum"
  if [ "$(sha256 "$tmp/$pkg")" != "$(tr -d ' \n' < "$tmp/sum")" ]; then
    echo "checksum mismatch for $pkg" >&2; rm -rf "$tmp"; exit 1
  fi
  rm -rf "$root/.tools/go"; mkdir -p "$root/.tools"
  tar -C "$root/.tools" -xzf "$tmp/$pkg"
  rm -rf "$tmp"
}

GO=""
if [ -x "$root/.tools/go/bin/go" ] && go_ok "$root/.tools/go/bin/go"; then GO="$root/.tools/go/bin/go"
elif command -v go >/dev/null && go_ok "$(command -v go)"; then GO="$(command -v go)"
else install_go; GO="$root/.tools/go/bin/go"; go_ok "$GO" || { echo "downloaded Go does not run" >&2; exit 1; }
fi
echo "go: $GO ($("$GO" env GOVERSION))"
command -v claude >/dev/null || echo "warn: claude CLI not on PATH" >&2
# hooks run as `python3 .claude/hooks/<name>.py` (settings.json); require python3 >= 3.8
if ! command -v python3 >/dev/null || ! python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 8) else 1)' 2>/dev/null; then
  echo "python3 (>= 3.8) not found: hooks need it. Install it (mac: xcode-select --install or brew install python; linux: apt/dnf install python3) and re-run." >&2
  exit 1
fi
echo "python: $(command -v python3) ($(python3 -V 2>&1))"

cd "$root"
mkdir -p bin
# Go module lives in src/main/intt; outputs stay under the repo root (bin/ layout: <exe>/../.claude)
CGO_ENABLED=0 "$GO" -C "$root/src/main/intt" build -trimpath -o "$root/bin/chaude" ./cmd/chaude
if [ "$cross" = 1 ]; then
  mkdir -p dist
  for t in darwin/arm64 darwin/amd64 linux/amd64 linux/arm64 windows/amd64 windows/arm64; do
    ext=""; [ "${t%/*}" = windows ] && ext=".exe"
    CGO_ENABLED=0 GOOS="${t%/*}" GOARCH="${t#*/}" "$GO" -C "$root/src/main/intt" build -trimpath -o "$root/dist/chaude-${t%/*}-${t#*/}${ext}" ./cmd/chaude
  done
  echo "cross builds: dist/"
fi
bin/chaude gen-settings "$os" --src .claude/settings.json --out .claude/settings.json
[ "$os" = linux ] && echo "state dir: ${XDG_STATE_HOME:-$HOME/.local/state}/claude-harness"
[ "$os" = mac ] && echo "state dir: $HOME/.claude-harness"
echo "ok ($os/$goarch): bin/chaude"
