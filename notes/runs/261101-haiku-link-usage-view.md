task: 출력 링크·경로 색상 + /usage 전체 화면 스크롤 뷰
commit: daaa807 (master, pushed 88e773f..daaa807)
changed: internal/harness/links.go, internal/harness/links_test.go, internal/harness/usageview_test.go, internal/harness/tui.go, internal/harness/agentview.go
fix_during_run: tui.go usageMsg handler clamps usageTop after text set (short refresh kept stale offset; test caught it)
checks:
  go vet ./... -> clean (rc 0)
  go test ./... -> ok internal/harness, scripts, settings (rc 0)
  GOOS=windows|linux|darwin go build ./... -> rc 0 each
  new tests: TestLinkSpansDetects, TestLinkSpansProseNegatives, TestLinkSpansOverlapAndBoundary, TestLinkifyPreservesVisibleText, TestWrapLinkedSkipsFence, TestUsageViewScrollBounds, TestUsageViewWheelAndRefreshKeepScroll, TestUsageViewEscCloses, TestUsageOpenNotInline -> all PASS
ci: run 41299158849 https://github.com/int-ttt/chaude/actions/runs/41299158849 -> success (ubuntu, windows, macos)
doubts:
  trailer: brief says Co-Authored-By Claude Opus 5.5; later system attribution reminder says Haiku 5.5. Kept Opus (user brief takes precedence). Amend if Haiku wanted.
  record/pastes/239c5fba.md has no spec (only old CI hand-back); brief used as spec.
  wheel scroll step fixed at 3 rows.
  usageView live /usage fetch not exercised (stubbed usageFetch).
untracked_left: notes/runs/*, .chaude/ (not committed)
