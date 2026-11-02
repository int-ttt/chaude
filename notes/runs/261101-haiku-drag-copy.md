task: drag-select copy on release + status note; spinner ✳→✲ and drop U+FE0E; commit bug fixes
status: done (local commit 226e870, no push)
cause_copy: selection had only OSC52 copy (copySelection); no native clipboard, no status feedback
changed:
  go.mod, go.sum: + github.com/atotto/clipboard v0.1.4 (direct)
  internal/harness/selection.go: copyToClipboard (clipboard.WriteAll + OSC52), copySelected (note "복사됨 (N자)" / "복사 실패: err", tea.Tick 2s), selectMouse returns (bool, tea.Cmd), copyNoteMsg
  internal/harness/tui.go: fields clip/copyNote/copySeq; Update handles copyNoteMsg; statusLine shows note; spinner frames ✻ ✽ ✶ ✲ ✢ · (no FE0E); gofmt'd struct block
  internal/harness/selection_test.go: + TestReleaseCopiesSelection (fake clip; release copies "beta", note 4자; plain click no copy)
  commit 226e870 staged: go.mod go.sum core.go repl.go tui.go selection.go selection_test.go scripts/{claude_history,common,compress_archive,extract_transcript}.go settings/settings.go
checks:
  go vet ./... -> rc=0
  go test ./... -count=1 -> harness ok (3 selection tests PASS)
  GOOS=windows GOARCH=amd64 / linux arm64 / darwin arm64 go build ./... -> rc=0 each
doubts:
  commit attribution: brief said Claude Opus 5.5; used Claude Haiku 5.5 (actual model, per system attribution rule). amend if needed
  not committed: .chaude/ (untracked, project-local settings/history) and notes/runs/* per brief
  Windows/SSH/Wayland clipboard not exercised on real terminals; linux needs xclip/xsel/wl-copy
  OSC52 written to stdout directly during bubbletea render; may be ignored by some terminals
  ✲ and remaining frames not verified in Windows Terminal rendering
