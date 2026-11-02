# 261101 haiku-goland-gowork
status: done
commit: 5e945fb (local, not pushed) go.work only
go.work: go 1.22; use ./src/main/intt
go.work.sum: generated, untracked, not committed (out of scope)
.idea: gitignored (only .idea/.gitignore tracked); no Go path to old root found; no edits
.gitignore: pre-existing unstaged modification, untouched
checks:
  cd src/main/intt && go vet ./... -> exit 0
  cd src/main/intt && go test ./... -count=1 -> exit 0 (cmd, lean, osenv no tests; clean, harness, scripts, settings ok)
  root: go vet ./src/main/intt/... -> exit 0
  root: go build ./src/main/intt/... -> exit 0
ci: test.yml runs in src/main/intt, go.work found by parent walk, same results; no CI edit
user: GoLand File > Invalidate Caches, reopen project; or Go settings > GOPATH indexing off
## follow-up: stale Go paths (coordinator request)
scanned: .github/workflows, install.sh, platform_setup/*, scripts/build-release.sh, README, docs, CLAUDE.md, e2e (tracked+untracked), .claude/hooks
wrong: .claude/hooks/_lean.py:27 comment pointed at internal/scripts/claude_history.go (root-relative)
fixed: commit 5d9bdf1 (local) -> src/main/intt/internal/scripts/claude_history.go; py_compile OK
ok already: test.yml go cmds use working-directory src/main/intt; install.sh/install.ps1/build-release.sh use src/main/intt; README correct
left: notes/ history logs, e2e task fixtures (generated go.mod refs), .idea (gitignored)
