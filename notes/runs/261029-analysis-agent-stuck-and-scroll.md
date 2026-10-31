# agent stuck as running + scrollable agent list
status: fixed, go build ./... OK, go test ./internal/harness/... OK (2 new tests, 1 updated)

## root cause (evidence from code; not reproduced live -> inference on exact trigger)
- agent done-marking only happened in 2 places: idleEvent (repl.go ~298-312, active child with no turn) and runTurn tool_result only (repl.go ~527).
- gap 1: runTurn "user" case handled tool_result only; <task-notification> (string content / text block) arriving during a user turn was ignored -> background haiku-scout stays "실행 중".
- gap 2: livechild.go parkedIdleEvent (0.8.0 parked-live children) never handled "user" events -> notifications of parked child's agents never recorded. Also sweepIdle (idlesweep.go:~44 `busy`) treats such child as busy forever -> never expired.
- fallback finishOldestBackground (repl.go idleFinish) marks only ONE agent per idle result and only on active child -> 12 leftovers accumulate.
- inference: in a session with many background haiku-scout launches + model switches/turns, notices were missed this way.

## fix
- drain.go: new agentTracker.onUserEvent(ev) (tool_results + notifications, returns notif seen).
- repl.go runTurn: user case calls r.tracker.onUserEvent(ev).
- livechild.go parkedIdleEvent: handle "user" via onUserEvent; on result without notification, finishOldestBackground (same fallback as idleFinish). children.go: liveChild.sawNotif field.
- scroll: tui.go rowsA windows the list (limit-1 rows + indicator "… a-b/N (마우스 휠로 스크롤)"), tuiModel.panOff; agentview.go panelWheel handles wheel when mouse Y is on the panel area (below input box); wheel elsewhere unchanged.
- tests: panel_test.go TestPanelScrollWindow, TestUserEventNotificationFinishesBackground; TestPanelMoreRowAndSmallHeight updated to new indicator text.

## open
- agents already stuck in the running TUI need restart (in-memory state). Interrupted-on-exit store logic unchanged.
- panOff is shared by fg and bg panels (each clamped independently).
- if still stuck after this: check whether notification text lacks <tool-use-id> (notifIDRe) in real stream; no live sample checked.
