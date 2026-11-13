#!/bin/sh
# chaude installer (macOS arm64 only / Linux amd64 + arm64). Standalone: installs prebuilt release binaries only
# (no source build, no repo checkout, so git and go are not needed). Public repo: no token required.
#   curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash
# Windows: install.ps1 (irm ... | iex).
# Installs to ~/.local/bin/chaude (user dir, no sudo). Adds ~/.local/bin to PATH in your shell rc file once, only when it is missing.
# Env: CHAUDE_VERSION (default latest, e.g. v0.11.11), CHAUDE_REPO.
# An older /usr/local/bin/chaude is not removed here (removing it needs sudo); the script prints the command to remove it.
# Optional token (only for rate limits or private forks): GH_TOKEN, else GITHUB_TOKEN, else `gh auth token`.
# A rejected token is dropped and the request is retried anonymously.
set -eu
REPO="${CHAUDE_REPO:-int-ttt/chaude}"
VERSION="${CHAUDE_VERSION:-latest}"
DIR="${HOME}/.local/bin"
API="https://api.github.com"

for t in curl tar awk sed install mktemp; do
  command -v "$t" >/dev/null 2>&1 || { echo "error: required tool not found: ${t} (install it and run again)" >&2; exit 1; }
done
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
  echo "error: required tool not found: sha256sum or shasum (install coreutils or perl-shasum and run again)" >&2; exit 1
fi

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$os" in
  darwin|linux) ;;
  mingw*|msys*|cygwin*)
    # Git Bash / MSYS / Cygwin: hand off to the Windows installer through powershell.exe
    command -v powershell.exe >/dev/null 2>&1 || { echo "error: powershell.exe not found. Run install.ps1 in PowerShell instead." >&2; exit 1; }
    echo "Windows detected: running install.ps1 through PowerShell" >&2
    exec powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex" ;;
  *) echo "error: unsupported OS: ${os}" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64) arch=amd64 ;;
  arm64|aarch64) arch=arm64 ;;
  *) echo "error: unsupported architecture: ${arch}" >&2; exit 1 ;;
esac
if [ "$os" = darwin ] && [ "$arch" = amd64 ]; then
  echo "error: unsupported: Intel Mac (darwin/amd64). chaude supports Apple Silicon (arm64) Macs only." >&2; exit 1
fi
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
  if [ -n "$token" ]; then curl -fsSL --retry 3 -H "Accept: application/vnd.github+json" -H "Authorization: Bearer ${token}" "$1"
  else curl -fsSL --retry 3 -H "Accept: application/vnd.github+json" "$1"; fi
}
# download a release asset by its API url (octet-stream; follows the redirect to storage)
api_asset() {
  if [ -n "$token" ]; then curl -fsSL --retry 3 -H "Accept: application/octet-stream" -H "Authorization: Bearer ${token}" -o "$2" "$1"
  else curl -fsSL --retry 3 -H "Accept: application/octet-stream" -o "$2" "$1"; fi
}
# print the API url of the asset whose name is $1 (asset objects list "url" before "name")
asset_url() {
  printf '%s\n' "$json" | awk -v n="$1" '
    /"url": *"[^"]*\/releases\/assets\/[0-9]+"/ { s=$0; sub(/.*"url": *"/, "", s); sub(/".*/, "", s); u=s }
    index($0, "\"name\": \"" n "\"") { print u; exit }'
}
# read release JSON; a rejected token is dropped (token="") and the read is retried anonymously
fetch_release() {
  json="$(api_get "$rel" 2>/dev/null)" && return 0
  [ -n "$token" ] || return 1
  echo "warning: GitHub rejected the token; retrying without it" >&2
  token=""
  json="$(api_get "$rel" 2>/dev/null)"
}

# run "$@" up to 3 times, waiting 2 s then 4 s: the release job publishes the release before its assets, so a read can come too early
retry() {
  for wait in 0 2 4; do
    [ "$wait" -eq 0 ] || { echo "retrying in ${wait} s..." >&2; sleep "$wait"; }
    "$@" && return 0
  done
  return 1
}
# read the release, its tag and the urls of the tar and checksums assets for this OS/arch (sets tag, name, tar_url, sums_url)
read_release() {
  fetch_release || return 1
  tag="$(printf '%s\n' "$json" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$tag" ] || return 1
  name="chaude_${tag#v}_${os}_${arch}.tar.gz"
  tar_url="$(asset_url "$name")"
  sums_url="$(asset_url checksums.txt)"
  [ -n "$tar_url" ] && [ -n "$sums_url" ]
}

if [ "$VERSION" = latest ]; then rel="${API}/repos/${REPO}/releases/latest"; else rel="${API}/repos/${REPO}/releases/tags/${VERSION}"; fi
retry read_release || { echo "error: release ${VERSION} of ${REPO} is incomplete after 3 tries: the release, its chaude_*_${os}_${arch}.tar.gz or checksums.txt is missing. Check the version name and your network; if GitHub rate-limited you (HTTP 403), set GH_TOKEN. If the release is complete but has no ${os}/${arch} archive, there is no prebuilt binary. Fallback: docs/guide/cli.md (install failures)." >&2; exit 1; }

echo "downloading ${name} (${tag})"
retry api_asset "$tar_url" "${tmp}/${name}" || { echo "error: download failed: ${name}" >&2; exit 1; }
retry api_asset "$sums_url" "${tmp}/checksums.txt" || { echo "error: download failed: checksums.txt" >&2; exit 1; }

# checksums.txt is required: never install an unverified binary
want="$(awk -v f="$name" '$2 == f {print $1; exit}' "${tmp}/checksums.txt")"
[ -n "$want" ] || { echo "error: ${name} is not listed in checksums.txt" >&2; exit 1; }
got="$(sha256 "${tmp}/${name}")"
[ "$want" = "$got" ] || { echo "checksum mismatch for ${name}" >&2; exit 1; }
tar -xzf "${tmp}/${name}" -C "$tmp" chaude

# user dir: no sudo. Creating it or writing to it must work for this user.
mkdir -p "$DIR" || { echo "error: cannot create ${DIR}. Check your home folder permissions and run again." >&2; exit 1; }
[ -w "$DIR" ] || { echo "error: ${DIR} is not writable by this user. Fix its permissions and run again (no sudo needed)." >&2; exit 1; }
if [ -e "${DIR}/chaude" ]; then
  old="$("${DIR}/chaude" --version 2>/dev/null | sed -n 's/^chaude \(.*\)$/\1/p' | head -n1 | tr -d '\n' | tr -c 'A-Za-z0-9._-' '_')"
  old="${old:-unknown}"
  mv -f "${DIR}/chaude" "${DIR}/chaude.${old}.bak"
  echo "backed up previous binary to ${DIR}/chaude.${old}.bak"
fi
install -m 755 "${tmp}/chaude" "${DIR}/chaude"
[ "$os" = darwin ] && xattr -d com.apple.quarantine "${DIR}/chaude" 2>/dev/null || true

# PATH: add DIR to a shell rc file once, only when DIR is not on PATH now. The exact line is never added twice.
path_line='export PATH="$HOME/.local/bin:$PATH"'
path_changed=""
case ":$PATH:" in
  *":${DIR}:"*) ;;
  *)
    case "${SHELL:-}" in
      */bash) if [ "$os" = darwin ]; then rc="$HOME/.bash_profile"; else rc="$HOME/.bashrc"; fi ;;
      */zsh) rc="$HOME/.zshrc" ;;
      *) if [ "$os" = darwin ]; then rc="$HOME/.zshrc"; else rc="$HOME/.profile"; fi ;;
    esac
    if [ -f "$rc" ] && grep -qxF "$path_line" "$rc"; then
      :
    else
      printf '\n# added by the chaude installer\n%s\n' "$path_line" >> "$rc"
      echo "PATH: added ${DIR} to ${rc}"
    fi
    path_changed=1 ;;
esac

echo "installed: ${DIR}/chaude"
"${DIR}/chaude" --version
if [ -n "$path_changed" ]; then
  echo "Open a new terminal (or run: ${path_line}) to use chaude."
fi
# older installs: /usr/local/bin (sudo copy) is not removed by this script; print the command instead
if [ -e /usr/local/bin/chaude ]; then
  echo "note: an older chaude is still at /usr/local/bin/chaude. Remove it so the new one is used:" >&2
  echo "  sudo rm -f /usr/local/bin/chaude" >&2
fi
command -v claude >/dev/null 2>&1 || echo "warning: claude (Claude Code CLI) not found on PATH. chaude needs it to run. Install Claude Code and open a new terminal." >&2
exit 0
