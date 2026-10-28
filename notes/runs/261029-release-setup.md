# release setup (261029)
files: .github/workflows/release.yml (tag v* -> scripts/build-release.sh -> gh release create, GITHUB_TOKEN only), scripts/build-release.sh, install.sh, README.md (설치 section appended)
archives: chaude_<ver>_darwin_arm64.tar.gz only (Intel/linux dropped per user) + checksums.txt; install.sh errors on non-arm64 Mac
verified: sh -n install.sh & build script OK; local build via MAIN_PKG=./cmd/harness darwin arm64+amd64 OK (tar contains `chaude`). NOT verified: shellcheck/goreleaser not installed; yaml not linted (no pyyaml); install.sh not run end-to-end (needs a real release)
assumptions: main pkg ./cmd/chaude (default, override MAIN_PKG); ldflag -X main.version must exist in main (else --version fails); go.mod needs go version-file (exists)
user todo: set OWNER/REPO in install.sh + README (or CHAUDE_REPO); repo must be public for curl|sh; push to GitHub, then tag vX.Y.Z; no secrets needed; Actions write perms default via workflow permissions
caveat: binary unsigned; quarantine only cleared by xattr, curl downloads don't get quarantine anyway

## update: v0.6.0 / claude-lean successor
repo: reuse int-ttt/claude-lean (private, main; releases v0.5.1-v0.5.3, latest v0.5.3 "Claude Lean 0.5.3"); no git remote configured locally, tags none
version: first chaude release = v0.6.0 (claude-lean .claude/claude-lean.version=0.5.3 untouched); CHANGELOG.md added, README note added
install.sh: default CHAUDE_REPO=int-ttt/claude-lean; uses `gh release download` if gh authed (private ok), else curl (public only); shasum verify; sh -n OK; not run e2e
to release (user): git remote add origin git@github.com:int-ttt/claude-lean.git (or push to it), commit these files, push main, `git tag v0.6.0 && git push origin v0.6.0`; main must have `var version` + --version + cmd/chaude (post rename). Caveat: "latest" on that repo flips from claude-lean 0.5.3 to chaude 0.6.0 (old claude-lean installers that read latest would get chaude assets); main branch content of that repo is claude-lean package -> pushing harness there replaces/merges history, user decision
