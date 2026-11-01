#!/bin/sh
# chaude installer (macOS / Linux, amd64 + arm64). Standalone: no repo checkout needed.
#   curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash
# Windows: irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
# Env: CHAUDE_VERSION (default latest, e.g. v0.9.11), CHAUDE_INSTALL_DIR (default ~/.local/bin), CHAUDE_REPO.
# Private repo: if `gh` is installed and logged in, it is used for downloads.
set -eu
REPO="${CHAUDE_REPO:-int-ttt/chaude}"
VERSION="${CHAUDE_VERSION:-latest}"
DIR="${CHAUDE_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$os" in
  darwin|linux) ;;
  mingw*|msys*|cygwin*) echo "error: Windows is not handled here. Use install.ps1 (PowerShell)." >&2; exit 1 ;;
  *) echo "error: unsupported OS: ${os}" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64) arch=amd64 ;;
  arm64|aarch64) arch=arm64 ;;
  *) echo "error: unsupported architecture: ${arch}" >&2; exit 1 ;;
esac
case "$VERSION" in
  latest|v*) ;;
  *) VERSION="v${VERSION}" ;;
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
  name="$(cd "$tmp" && ls "chaude_"*"_${os}_${arch}.tar.gz" 2>/dev/null | head -n1 || true)"
else
  if [ "$VERSION" = latest ]; then
    VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
    [ -n "$VERSION" ] || { echo "could not resolve latest release of ${REPO} (private repo? install and log in with gh: gh auth login)" >&2; exit 1; }
  fi
  name="chaude_${VERSION#v}_${os}_${arch}.tar.gz"
  base="https://github.com/${REPO}/releases/download/${VERSION}"
  echo "downloading ${name}"
  curl -fsSL "${base}/${name}" -o "${tmp}/${name}"
  curl -fsSL "${base}/checksums.txt" -o "${tmp}/checksums.txt"
fi
[ -n "${name:-}" ] && [ -f "${tmp}/${name}" ] || { echo "download failed" >&2; exit 1; }

# checksums.txt is required: never install an unverified binary
[ -f "${tmp}/checksums.txt" ] || { echo "error: checksums.txt not found in release; refusing to install" >&2; exit 1; }
want="$(awk -v f="$name" '$2 == f {print $1; exit}' "${tmp}/checksums.txt")"
[ -n "$want" ] || { echo "error: ${name} is not listed in checksums.txt" >&2; exit 1; }
got="$(sha256 "${tmp}/${name}")"
[ "$want" = "$got" ] || { echo "checksum mismatch for ${name}" >&2; exit 1; }
tar -xzf "${tmp}/${name}" -C "$tmp" chaude

mkdir -p "$DIR"
if [ -e "${DIR}/chaude" ]; then
  old="$("${DIR}/chaude" --version 2>/dev/null | sed -n 's/^chaude \(.*\)$/\1/p' | head -n1 | tr -d '\n' | tr -c 'A-Za-z0-9._-' '_')"
  old="${old:-unknown}"
  mv -f "${DIR}/chaude" "${DIR}/chaude.${old}.bak"
  echo "backed up previous binary to ${DIR}/chaude.${old}.bak"
fi
install -m 755 "${tmp}/chaude" "${DIR}/chaude"
[ "$os" = darwin ] && xattr -d com.apple.quarantine "${DIR}/chaude" 2>/dev/null || true

case ":$PATH:" in
  *":${DIR}:"*) ;;
  *)
    # rc file by login shell: zsh -> ~/.zshrc; bash -> ~/.bash_profile (mac) or ~/.bashrc (linux); other -> ~/.profile
    case "${SHELL:-}" in
      */zsh) rc="$HOME/.zshrc" ;;
      */bash) if [ "$os" = darwin ]; then rc="$HOME/.bash_profile"; else rc="$HOME/.bashrc"; fi ;;
      *) rc="$HOME/.profile" ;;
    esac
    line="export PATH=\"${DIR}:\$PATH\""
    grep -qsF "$line" "$rc" || printf '\n# chaude\n%s\n' "$line" >> "$rc"
    echo "PATH hint: ${DIR} is not in PATH. Added to ${rc} (open a new terminal, or run: . ${rc})"
    PATH="${DIR}:$PATH"
    ;;
esac

echo "installed: ${DIR}/chaude"
"${DIR}/chaude" --version
