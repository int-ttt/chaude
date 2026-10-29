# 0.7.3 multiproc integration progress (2026-10-29)
status: partial
done:
- U1c idleDeadline (clients.go) + TestIdleDeadline: PASS
- U4b childSet/liveChild/closer/Park/Take/Expired/CloseAll (children.go) + TestChildSetParkTake, TestCloseAllParallel: PASS
- U3b flags --cache-ttl(1h) --idle-rotate(3599s) --clients-file; Args.CacheTTL/IdleRotate/ClientsFile; SessionOpts.CacheTTL in start(): build/vet/test PASS
- U6a pickChild (dispatch.go) + TestPickChild: PASS
remaining:
- U5 touchMain/idleEvent parked branch/DesignPath + TestTouchMainOnly, TestIdleEventFromParkedChild, TestDesignPathRecorded, TestDesignPathMissingSkipped: not started (active-slot rebinding, A3)
- U6b classifier different-model path + TestRouteKeepsPreviousChildLive, TestRouteSameModelWorkerUsesOldPath, TestRouteFallsBackWithoutRegistry: not started (depends U5)
- U7 idleDue + TestIdleDue: not started (depends U5)
- U8 tui sweepIdle + CloseAll on quit: not started (depends U7, U6b)
- TestReconcileKilledOwner already exists in clients_test.go (not re-verified this run)
acceptance: go build ./... BUILD_OK; go vet ./internal/harness/ VET_OK; go test ./... -count=1 ok chaude/internal/harness
uncommitted: yes (no commit requested)
