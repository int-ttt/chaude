task: clean-install CLI fixes (gen-settings, claude-history, compress-archive -h, extract-transcript slashes)
status: done (no commit)
cause:
  gen-settings: OS positional required; no args -> usage rc=2. Fixed: OS defaults to osenv.Name(); -h rc=0 with help; src read error message clarified.
  claude-history: no args -> printed empty line (rc=0). HistoryDir("") silently empty. Fixed: default proj = CLAUDE_PROJECT_DIR or cwd; not found -> stderr "기록 없음: ... (확인 위치: ...)" rc=1; -h supported (was treated as a path).
  compress-archive -h: no flag parsing, "-h" treated as dir -> silent. Fixed: newFS/parseArgs, usage on no args, "압축할 기록 없음" when nothing.
  -h for all scripts: newFS had no description. Fixed: helpText map in common.go, fs.Usage prints it.
  extract-transcript: printed "%s/INDEX.md" with raw out dir (mixed sep when --out is a Windows path). Fixed: filepath.ToSlash on printed INDEX path and not-found src.
changed:
  internal/scripts/common.go (helpText + newFS Usage)
  internal/scripts/claude_history.go
  internal/scripts/compress_archive.go
  internal/scripts/extract_transcript.go
  internal/settings/settings.go
checks:
  go vet ./... -> PASS (no output)
  go test ./... -> PASS (internal/harness ok; others no test files)
  GOOS=windows GOARCH=amd64 go build ./cmd/chaude -> PASS
  temp HOME, each subcommand -h and default args -> all print usage/message, rc as documented
  gen-settings windows --python "py -3" -> command=py, args ok
  extract-transcript --out "<tmp>\notes\distill" -> INDEX path printed with / only (mac run; Windows sep not exercised at runtime)
doubts:
  gen-settings default --out = stdout (not a file path). Changing to file default would clobber source; left as stdout.
  Windows runtime behavior for claude-history encoding not run on Windows.
  Other agent's uncommitted edits present (internal/harness/core.go repl.go tui.go selection.go); not touched.
  gofmt -l flags internal/harness/tui.go (pre-existing / other agent), not my files.
