verdict: PASS
blocking: 0
checks: go build ok · go vet ok · go test -count=1 ok · go test -race -count=3 ./internal/harness ok
scroll rule (notes/runs/scroll-bottom.md): kept; tui.go held/GotoBottom logic unchanged by diff (only sweepIdle hook + nil-sess guard added)
nil maps: r.parked lazily init (classifier.go:278), read/delete on nil ok; reg/children nil -> old path (route guard classifier.go, nil-safe reg methods)
same-model worker switch: routeChildren (switchchild.go) uses switchSession old path; covered by TestRouteSameModelWorkerUsesOldPath
park/resume: unbindActive sets parked first, bindChild clears last; TestUnbindParkedRace; start-fail rollback handled (switchchild.go switchChild)
idle sweep: clockMu guards lastAPI/ttlSeen/idleFiredFor; busy parked kept; skipped while busy/perm prompt (tests)
non-blocking notes:
- low tui.go:383: m.submit unbuffered send in Update; same pattern as existing sends, guarded by !m.busy; ok
- low tui.go:1146: "/idle-rotate" typed by user would also trigger idleRotateNow (no-op if no sess); harmless
- low idlesweep.go:44-58: sweepIdle does registry file I/O on UI goroutine per tick; only when children parked
- low switchchild.go switchChild start-fail path: old closed branch calls r.start() ignoring err; user sees error already
clients.go: unmodified in git status (tracked, no diff)
