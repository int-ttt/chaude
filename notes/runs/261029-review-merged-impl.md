# review: merged impl vs 261029-plan-merged.md
verdict: PASS (0 blocking); go build ok, go test ./internal/harness ok

## BLOCKING
none

## non-blocking
- .claude/agents/haiku-{scout,researcher,...5 files}.md "Return ≤5 lines: ..." line coexists with new "Final message = ≤3 lines, ≤300 chars": conflicting limits; fix: drop/align "≤5" to "≤3" (or say Return = same as final message)
- tui.go:392 lastTool set from every toolMsg incl. subagent-nested tools (if forwarded); fix: acceptable per D4, verify no parent_tool_use_id leakage
- tui.go mainStatusText: elapsed not reset on turn end by explicit code (line 431 busy=false) but guarded by m.busy -> OK; no issue
- localcmd.go: /help,/status local only in plain loop, after "> " echo; /exit placed before echo (no echo, fine). /status mid-turn reads r.lastCtx unsynchronised (single-thread loop, OK)
- localcmd_test.go: "/help" substring check is weak (echo "> /help" alone satisfies it); fix: assert "/rotate" or desc only present in help ("show commands and keys" already does, so fine)
- specter_test.go: covers busy path only; plan said idle path unchanged -> add assert idle (busy=false) has no " · 2m"
- drain.go onToolResultText: marks Background only if !Background; correct; test covers launch, fg finish, notification. Plan-conformant.
- repl.go: effectiveThreshold/opusSonnetThreshold + -threshold 80000 default are pre-existing threshold work (ignored)
