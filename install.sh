#!/bin/sh
# chaude installer (macOS / Linux, amd64 + arm64): curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/main/install.sh | sh
# Windows: use platform_setup\install.ps1 instead.
set -eu
REPO="${CHAUDE_REPO:-int-ttt/chaude}"
VERSION="${CHAUDE_VERSION:-latest}"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$os" in
  darwin|linux) ;;
  mingw*|msys*|cygwin*) echo "error: Windows is not installed by this script. Use platform_setup\\install.ps1 (PowerShell)." >&2; exit 1 ;;
  *) echo "error: unsupported OS: ${os}" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64) arch=amd64 ;;
  arm64|aarch64) arch=arm64 ;;
  *) echo "error: unsupported architecture: ${arch}" >&2; exit 1 ;;
esac

sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
  # works for private repos (uses your gh login)
  tagarg=""; [ "$VERSION" = latest ] || tagarg="$VERSION"
  echo "downloading via gh from ${REPO}"
  # shellcheck disable=SC2086
  gh release download $tagarg -R "$REPO" -p "chaude_*_${os}_${arch}.tar.gz" -p checksums.txt -D "$tmp" --clobber
  name="$(cd "$tmp" && ls "chaude_"*"_${os}_${arch}.tar.gz" | head -n1)"
else
  # public repos only
  if [ "$VERSION" = latest ]; then
    VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
    [ -n "$VERSION" ] || { echo "could not resolve latest release of ${REPO} (private repo? install and log in with gh: gh auth login)" >&2; exit 1; }
  fi
  name="chaude_${VERSION#v}_${os}_${arch}.tar.gz"
  base="https://github.com/${REPO}/releases/download/${VERSION}"
  echo "downloading ${name}"
  curl -fsSL "${base}/${name}" -o "${tmp}/${name}"
  curl -fsSL "${base}/checksums.txt" -o "${tmp}/checksums.txt" 2>/dev/null || true
fi
[ -f "${tmp}/${name}" ] || { echo "download failed" >&2; exit 1; }
if [ -f "${tmp}/checksums.txt" ]; then
  want="$(grep " ${name}\$" "${tmp}/checksums.txt" | awk '{print $1}')"
  got="$(sha256 "${tmp}/${name}")"
  [ -n "$want" ] && [ "$want" = "$got" ] || { echo "checksum mismatch" >&2; exit 1; }
fi
tar -xzf "${tmp}/${name}" -C "$tmp" chaude

if [ -n "${CHAUDE_INSTALL_DIR:-}" ]; then dir="$CHAUDE_INSTALL_DIR"
elif [ -d /usr/local/bin ] && [ -w /usr/local/bin ]; then dir=/usr/local/bin
else dir="$HOME/.local/bin"; fi
mkdir -p "$dir"
install -m 755 "${tmp}/chaude" "${dir}/chaude"
[ "$os" = darwin ] && xattr -d com.apple.quarantine "${dir}/chaude" 2>/dev/null || true

case ":$PATH:" in
  *":${dir}:"*) ;;
  *)
    # rc file by login shell: zsh -> ~/.zshrc; bash -> ~/.bash_profile (mac) or ~/.bashrc (linux); other -> ~/.profile
    case "${SHELL:-}" in
      */zsh) rc="$HOME/.zshrc" ;;
      */bash) if [ "$os" = darwin ]; then rc="$HOME/.bash_profile"; else rc="$HOME/.bashrc"; fi ;;
      *) rc="$HOME/.profile" ;;
    esac
    line="export PATH=\"${dir}:\$PATH\""
    grep -qsF "$line" "$rc" || printf '\n# chaude\n%s\n' "$line" >> "$rc"
    echo "added ${dir} to PATH in ${rc} (open a new terminal or: . ${rc})"
    PATH="${dir}:$PATH"
    ;;
esac

echo "installed: ${dir}/chaude"
"${dir}/chaude" --version
