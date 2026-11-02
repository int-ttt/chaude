task: chaude clean [dir] [--yes]
agent: haiku-coder
date: 261101
status: done
commit: bf54df4 (local, not pushed)
changed:
  - src/main/intt/internal/clean/clean.go (new)
  - src/main/intt/internal/clean/clean_test.go (new)
  - src/main/intt/cmd/chaude/main.go (clean case + usage line + import)
manifest_source: git 28d26c6:internal/install/install.go + tracked .claude/{agents,hooks,skills}
checks:
  - gofmt -l ./internal/clean ./cmd/chaude -> empty
  - go vet ./... (src/main/intt) -> exit 0
  - go test ./... -count=1 (src/main/intt) -> clean ok, harness ok, scripts ok, settings ok
  - clean tests: TestDryRunChangesNothing, TestApplyRemovesManifestAndBacksUp, TestApplyKeepsNotesRecordAndUserSettings, TestClaudeMdWithoutBlockUntouched, TestClaudeMdOnlyBlockIsDeleted, TestBrokenMarkersFailBeforeAnyChange, TestManifestMatchesRepo -> PASS
  - go run ./cmd/chaude clean <repo root> (dry-run only) -> 30 targets, nothing changed, git status of .claude and CLAUDE.md empty
not_run: --yes against the real repo (tests use temp fixtures only)
doubts:
  - install also wrote .claude/claude-lean.version, lean-off, chaude-mode, bin/chaude, chaude-start.command/.bat, notes templates, logs/, .gitignore lines. Not in brief scope, not removed.
  - brief says Co-Authored-By Opus 5.5; commit uses Haiku 5.5 per system reminder.
  - dry-run on repo counts 11 settings hook entries (earlier summary said 10).
  - .chaude-backup- not checked against .gitignore (not verified).
