#!/usr/bin/env bash
# mac + linux: make sure Rust (cargo) is available, check python3 (Claude Code hooks are .claude/hooks/*.py),
# build bin/chaude from crates/cli, write .claude/settings.json for this OS.
# Usage: platform_setup/install.sh     (cross-platform release archives are built by .github/workflows/release.yml)
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
for a in "$@"; do
  case "$a" in -h|--help) sed -n 2,4p "$0"; exit 0;; *) echo "unknown option: $a" >&2; exit 2;; esac
done

case "$(uname -s)" in Darwin) os=mac;; Linux) os=linux;; *) echo "unsupported: $(uname -s)" >&2; exit 1;; esac
case "$(uname -m)" in x86_64|amd64) goarch=amd64;; arm64|aarch64) goarch=arm64;; *) echo "unsupported arch: $(uname -m)" >&2; exit 1;; esac

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found: install Rust from https://rustup.rs (rust-toolchain.toml pins the version) and re-run." >&2
  exit 1
fi
echo "cargo: $(command -v cargo) ($(cargo --version))"
command -v claude >/dev/null || echo "warn: claude CLI not on PATH" >&2
# hooks run as `python3 .claude/hooks/<name>.py` (settings.json); require python3 >= 3.8
if ! command -v python3 >/dev/null || ! python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 8) else 1)' 2>/dev/null; then
  echo "python3 (>= 3.8) not found: hooks need it. Install it (mac: xcode-select --install or brew install python; linux: apt/dnf install python3) and re-run." >&2
  exit 1
fi
echo "python: $(command -v python3) ($(python3 -V 2>&1))"

cd "$root"
cargo build --release --locked -p chaude-cli
mkdir -p bin
cp target/release/chaude bin/chaude
bin/chaude gen-settings "$os" --src .claude/settings.json --out .claude/settings.json
[ "$os" = linux ] && echo "state dir: ${XDG_STATE_HOME:-$HOME/.local/state}/claude-harness"
[ "$os" = mac ] && echo "state dir: $HOME/.claude-harness"
echo "ok ($os/$goarch): bin/chaude"
