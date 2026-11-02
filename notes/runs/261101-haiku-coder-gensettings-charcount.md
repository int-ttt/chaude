task: gen-settings default path + charcount BOM/CRLF
status: done (local commit f345493, no push)
cause:
  gen-settings: default --src = <exe>/../.claude/settings.json only; ignores cwd. Binary outside <project>/bin (go run, copied, ~/.local/bin) -> reads <tmp>/.claude -> rc=1. Reproduced on mac build from /var/folders tmp dir.
  charcount: BOM kept in string -> +1 rune, +3 bytes. stdin path normalized CRLF only on windows (runtime.GOOS gate) -> CRLF counted on linux/mac stdin.
changed:
  internal/settings/settings.go: defaultSrc(projDir, cwd, exe): CLAUDE_PROJECT_DIR > exe-rel if exists > cwd/.claude if exists > exe-rel (error)
  internal/settings/settings_test.go (new): TestDefaultSrcPrecedence
  internal/scripts/charcount.go: TrimPrefix BOM, CRLF/CR->LF on all OS and stdin; runtime import removed
  internal/scripts/charcount_test.go (new): BOM+CRLF == plain LF; BOM-only stripped
  not used: osenv.StateDir (state dir is not the project settings source)
checks:
  gofmt -l internal/settings internal/scripts -> clean
  go vet ./... -> PASS
  go test ./... -> PASS (harness, scripts, settings ok)
  go test -run 'Charcount|DefaultSrc' -v -> PASS x3
  GOOS=linux|windows|darwin go build ./cmd/chaude -> PASS x3
  GOOS=windows go test -c ./internal/settings ./internal/scripts -> PASS
  e2e: charcount BOM+CRLF = 16 (file and stdin); gen-settings from empty dir -> rc=1 with concrete path
doubts:
  Linux/Windows runtime not executed; path logic tested on darwin host with filepath only (no windows-style path strings).
  commit trailer: brief said Opus 5.5; model is Haiku 5.5 -> used brief text verbatim, verify.
  notes/runs file untracked, not committed.
