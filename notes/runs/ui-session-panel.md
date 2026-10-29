# ui-session-panel
step1: /tmp/chaude-nocursor built with syncCursor early-return (curCol=0); tui.go restored from copy, git diff showed no leftover before steps 2-3
step2 (생각중...): internal/harness/tui.go render(): "생각중..." shown while tracker.running()>0 and no text; gone when agent done (tracker marks Done on tool_result/notification)
step3 (panel): tui.go panelLines()+View()/layout()/syncCursor() (panelH rows between input box and status line, max 4 + "+N more"); drain.go agentRec.Prompt (first prompt line), sortedRunning(), firstLine()
test: internal/harness/panel_test.go TestSessionPanelAndThinking
result: go build ./... ok, go test ./... ok
unverified: real terminal rendering (cursor offset with panel, JetBrains), background-agent completion path only by code reading; "session" interpreted as delegated agents
