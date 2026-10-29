# sonnet-analyst multiproc S units
date: 261029
units: U5a U5b U5d U6b U7b U8 = done (uncommitted; no commit/tag/push/version bump)
check: go build ./... PASS; go vet ./internal/harness/ PASS; go test ./... -count=1 PASS; go test -race (TouchMain|IdleEventParked|...|Sweep|ScrollBottom|WheelScroll|UnbindParkedRace|Route|IdleDue) PASS

## files
new: internal/harness/livechild.go idlesweep.go switchchild.go multiproc_s_test.go
edit: core.go (Session.dead, Dead()) children.go (liveChild fields) repl.go classifier.go (classifyFn seam, routeChildren hook) tui.go (closeAll on quit, /status nil guard, TickMsg sweepIdle, "/idle-rotate")

## decisions
expiredZero: spec U7b pseudocode inverted. expiredZero(d)=d>0 means "can expire"; idleDue/sweep use !expiredZero -> never due. 
idleDue: fires once per lastAPI; zero lastAPI -> false. Boundary is the deadline itself (idleDeadline for 1h = idleRotate), test uses deadline-1s false / deadline true, not plan's 3598/3599.
warmDeadline: neverCold when idleRotate=0 (isWarm false for deadline 0).
sweepIdle: busy parked child (running agents) not closed (spec said drain; retire() drains on route-out only). Expiry notice via goroutine (Send in Update deadlocks).
unbindActive: no longer rewrites Sess/Tracker/Model on r.cur (race with idle callback found by -race test).
same-model haiku worker switch keeps old switchSession path (routeChildren).
--idle-rotate help: "idle rotation runs in the TUI only".

## deviations
TestSweepDrainsBusyParked: adapted (busy parked child not closed).
TestRouteFallsBackWithoutRegistry: exercises route() with nil reg/children -> old rotate path.
TestPlainLoopCallsCloseAll: same body as TestQuitCallsCloseAll (closeAllChildren; plain loop/TUI quit wiring not driven end to end).
TestIdleNoticeKeepsHeldOffset: smoke only (wheel up, notice, tick, View); no offset assertion.
TestMakeSnapshotStates: snapInvalid snap string not asserted (came back empty with fake child).
TestNoResumeOfLiveSessionID: parked child is a stub closer, so assertion is weak.

## doubts
classifier.go gofmt unclean pre-existing.
haiku-coder not used (code tightly coupled).
