# agent status format
format: ●(colored) + "상태-모델-세션이름(작업 요약)" ; both fg panel and bg panel rows (tui.go rows())
locations: tui.go rows()+agentState/agentDot/agentStatusText; drain.go agentRec{Model,Name,Failed}; panel_test.go (TestAgentStatusFormat + updated asserts)
not changed: statusLine() ("agents N running" count kept); ~/.claude/settings.json has no statusLine entry
colors (256): running 220 yellow, done 42 green, failed 196 red, queued 245 gray
sources: model=Agent input.model (fallback session model r.a.Model); name=input.name else subagent_type else "agent"; summary=input.description (fallback first prompt line)
note: panels show only running agents (done drop out), so done/failed/queued colors are defined but not yet displayed; Failed never set (no failure signal in tracker)
line count unchanged (1 row per agent) -> cursor-row math untouched
verify: go build, go vet, go test -race ./internal/harness ok
