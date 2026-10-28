# REPL features (261029)
status: done, gofmt/vet/build/test pass
fix: repl.go atomic import was already present in tree; real issue was gofmt on repl.go
features:
- agent status: agentTracker.counts() (mutex-guarded) shown in TUI status line "agents N running (bg M)"
- jump-to-turn: esc esc on empty input opens rewind picker (rewindState); view+input only, child context kept
- history: up/down recall (inputHistory, cap 200) in TUI
- modes: default|acceptEdits|plan|auto via /mode, shift+tab (TUI); harness-side decideMode in askPermission
plain path: added /mode [name|next] (Repl.modeLine)
tests: TestInputHistory(+Cap), TestRewindState, TestAgentCounts, TestModeCycleAndParse, TestDecideMode, TestReplModeLine
not done: plain path has no history/rewind/agent-status line (cooked stdin); rewind does not truncate child session; no TUI-level interaction tests
