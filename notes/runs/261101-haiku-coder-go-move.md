status: done
commit: 2c8db19 (master, pushed a04bca6..2c8db19)
moved: go.mod go.sum cmd/ internal/ -> src/main/intt/ (git mv, 136 files incl. renames)
module: chaude (unchanged); bin/ and dist/ stay at repo root (settings.go <exe>/../.claude)
callers updated: test.yml(setup-go, working-directory, build -o ../../../bin), release.yml(go-version-file), build-release.sh(cd $MOD), install.sh(go -C), install.ps1(go -C), README.md(27,168), unit_test.go(samples path ../../../../../samples), .gitignore(/src/main/intt/chaude)
not moved: testdata/smoke (CI smoke + .gitattributes only)
left for report: .claude/hooks/_lean.py:27 comment (internal/scripts/claude_history.go), CHANGELOG/plan refs, tests/e2e-rebuild (own go.mod), e2e/run.sh (untracked, go usage unchecked)
checks:
  go vet ./... (src/main/intt) -> exit 0
  go test ./... -count=1 -> exit 0 (harness, scripts, settings ok)
  GOOS cross build darwin/arm64 linux/amd64 windows/amd64 -> ok
  scripts/build-release.sh 0.0.0-test in temp copy -> exit 0, 6 archives + checksums.txt
  platform_setup/install.sh in temp copy (/tmp/chaude-inst) -> exit 0, bin/chaude built
  python3 -I -m unittest discover -s tests/hooks -t tests/hooks -> 35 tests OK
  test.yml run 41301498930 -> success (ubuntu, windows, macos)
trailer: Claude Haiku 5.5 (system reminder); brief said Opus 5.5 -> conflict noted
untracked not committed: .github/workflows/e2e.yml (edited on disk, user WIP), e2e/, .chaude/, notes/runs/*
e2e/results/ .gitignore line left unstaged (user WIP)
