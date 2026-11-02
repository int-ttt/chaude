task: bg-agent rotate defer + result file
status: done (no commit)
changed: internal/harness/rotate_defer.go, internal/harness/agentresult.go, internal/harness/drain.go, internal/harness/repl.go, internal/harness/livechild.go, internal/harness/tui.go, internal/harness/bgagent_defer_test.go, .claude/agents/haiku-coder.md
not touched: internal/harness/history.go, internal/harness/history_file_test.go
defer: rotate() held while bg agents run; RotateDeferTimeout 10m (flag rotate-defer-timeout); after timeout rotate proceeds, handoff lists running agents
result file: notes/runs/<YYMMDD>-agent-<id>.md appended on <task-notification> or finishOldestBackground, once per agent (ResultDone)
checks: go build ./... -> BUILD_OK; go vet ./internal/harness/ -> VET_OK; go test ./internal/harness/... -count=1 -> ok chaude/internal/harness 0.855s
new tests: TestRotateDeferDue, TestRotateHeldWhileBackgroundAgentRuns, TestRotateDeferReleasedWhenAgentFinishes, TestDeferRotateTimeoutProceeds, TestBackgroundResultFileAndHandoff -> PASS
risk: history.go shows M in git status from before this task; not modified by me
