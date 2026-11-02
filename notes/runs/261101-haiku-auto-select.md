task: chaude auto permission default + transcript mouse drag-select (content-coord anchors)
status: done (no commit)
cause_perm: no --permission-mode passed to claude; harness mode defaulted to modeDefault (ask). No "auto-mod" string in Go code (grep: none; only in jsonl data files)
cause_select: no app-level selection; drag events ignored; terminal-native selection drifts with screen rows while streaming
changed:
  internal/harness/core.go: const claudePermissionMode="auto"; sessionArgs adds --permission-mode auto
  internal/harness/repl.go: NewRepl stores modeAuto as initial mode
  internal/harness/tui.go: fields lines/selc/dragging; refresh stores lines; mouse -> selectMouse; View highlights
  internal/harness/selection.go (new): textPos/textSel content coords, text(), highlightView(), selectMouse(), OSC52 copy
  internal/harness/selection_test.go (new): append-keeps-anchor, scroll highlight row, multi-line/reversed
checks:
  gofmt -l internal/harness: clean
  go vet ./...: clean
  go test ./internal/harness -run TestSelection -v: PASS (2)
  go test ./... -count=1: ok internal/harness, others no test files
  GOOS=windows GOARCH=amd64 go build ./...: ok
doubts:
  windows drag not verified on real terminal; mouse enabled unconditionally (WithMouseCellMotion, no platform guard)
  OSC52 copy written to os.Stdout directly; may be ignored by some terminals
  selection not cleared on rewind/new block; anchors are line indexes so re-wrap of streaming last block can shift highlight
  internal/scripts/* shows unstaged diff not made by me; untouched
