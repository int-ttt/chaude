#!/bin/sh
# chaude installer (macOS / Linux, amd64 + arm64). Standalone: installs prebuilt release binaries only
# (no source build, no repo checkout).
#   curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash
# Windows: install.ps1 (irm ... | iex).
# Env: CHAUDE_VERSION (default latest, e.g. v0.9.11), CHAUDE_INSTALL_DIR (default ~/.local/bin), CHAUDE_REPO.
# Auth (private repo): GH_TOKEN, else GITHUB_TOKEN, else `gh auth token`, else anonymous (public repo only).
set -eu
REPO="${CHAUDE_REPO:-int-ttt/chaude}"
VERSION="${CHAUDE_VERSION:-latest}"
DIR="${CHAUDE_INSTALL_DIR:-$HOME/.local/bin}"
API="https://api.github.com"

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

token=""
if [ -n "${GH_TOKEN:-}" ]; then token="$GH_TOKEN"
elif [ -n "${GITHUB_TOKEN:-}" ]; then token="$GITHUB_TOKEN"
elif command -v gh >/dev/null 2>&1; then token="$(gh auth token 2>/dev/null || true)"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# GitHub API GET (JSON)
api_get() {
  if [ -n "$token" ]; then curl -fsSL -H "Accept: application/vnd.github+json" -H "Authorization: Bearer ${token}" "$1"
  else curl -fsSL -H "Accept: application/vnd.github+json" "$1"; fi
}
# download a release asset by its API url (octet-stream; follows the redirect to storage without the token)
api_asset() {
  if [ -n "$token" ]; then curl -fsSL -H "Accept: application/octet-stream" -H "Authorization: Bearer ${token}" -o "$2" "$1"
  else curl -fsSL -H "Accept: application/octet-stream" -o "$2" "$1"; fi
}
# print the API url of the asset whose name is $1 (asset objects list "url" before "name")
asset_url() {
  printf '%s\n' "$json" | awk -v n="$1" '
    /"url": *"[^"]*\/releases\/assets\/[0-9]+"/ { s=$0; sub(/.*"url": *"/, "", s); sub(/".*/, "", s); u=s }
    index($0, "\"name\": \"" n "\"") { print u; exit }'
}

if [ "$VERSION" = latest ]; then rel="${API}/repos/${REPO}/releases/latest"; else rel="${API}/repos/${REPO}/releases/tags/${VERSION}"; fi
json="$(api_get "$rel" 2>/dev/null)" || { echo "error: cannot read release ${VERSION} of ${REPO} (private repo? set GH_TOKEN or run: gh auth login)" >&2; exit 1; }
tag="$(printf '%s\n' "$json" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
[ -n "$tag" ] || { echo "error: could not resolve release tag of ${REPO}" >&2; exit 1; }

name="chaude_${tag#v}_${os}_${arch}.tar.gz"
tar_url="$(asset_url "$name")"
[ -n "$tar_url" ] || { echo "error: ${name} not found in release ${tag} (no prebuilt binary for ${os}/${arch})" >&2; exit 1; }
sums_url="$(asset_url checksums.txt)"
[ -n "$sums_url" ] || { echo "error: checksums.txt not found in release ${tag}; refusing to install" >&2; exit 1; }

echo "downloading ${name} (${tag})"
api_asset "$tar_url" "${tmp}/${name}"
api_asset "$sums_url" "${tmp}/checksums.txt"

# checksums.txt is required: never install an unverified binary
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
