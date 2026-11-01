status: done
changed: install.sh (M), install.ps1 (new), README.md (M)
skipped: docs/claude-lean/README.md (claude-lean installer section, different product; not chaude)
check: bash -n install.sh -> PASS
check: install.sh HOME=tmp, GH_TOKEN, CHAUDE_VERSION=v0.9.11 (gh path) -> PASS, installed chaude 0.9.11, chaude -h exit 0
check: 2nd run piped via `cat install.sh | bash` -> PASS, backup chaude.0.9.11.bak, PATH rc line not duplicated
check: pwsh/powershell not installed -> install.ps1 parse NOT run
check: raw.githubusercontent.com/int-ttt/chaude/master/install.sh anonymous -> 404 (repo PRIVATE); public curl path NOT exercised end-to-end
fix: backup name trailing "_" from tr -c on newline (fixed, re-verified)
doubts: install dir default changed /usr/local/bin(if writable) -> ~/.local/bin per brief; checksums now mandatory (was optional); rc append kept
doubts: co-author line uses Opus 5.5 per brief (system reminder says Haiku 5.5)
ws: none

## follow-up (coordinator: private repo, token + API, no build/checkout)
- status: done (ps1 unverified)
- install.sh: API asset download with GH_TOKEN > GITHUB_TOKEN > gh auth token > anonymous; checksums.txt required; `tr -d '\n'` fix for backup name.
- install.ps1: rewritten to same flow; HttpClient AllowAutoRedirect=false, token only to api.github.com, storage URL fetched without token; ASCII-only; no exit/param.
- README.md: build-from-source moved to `## 개발자용`; private section uses token auth + gh api raw install.
- check: `bash -n install.sh` -> PASS
- check: real run HOME=tmp, gh token only (keychain symlinked) -> chaude 0.9.11, -h exit 0 (earlier, install.sh unchanged since)
- check: no-token fresh HOME -> clean error, exit 1
- check: `pwsh`/`powershell` not installed -> parse check NOT run
- check: install.ps1 ASCII-only grep -> PASS; no go build / platform_setup / checkout text in install.sh/install.ps1 -> PASS
- doubt: install.ps1 untested at runtime; Expand-Archive/Get-FileHash/HttpClient path unverified on 5.1 and 7.
- doubt: private section still fetches installer with `gh api` (installer only, not a checkout).
- doubt: co-author trailer: brief says Opus 5.5; harness reminder says Haiku 5.5. Used brief value.
