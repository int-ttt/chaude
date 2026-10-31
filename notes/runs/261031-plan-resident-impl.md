# plan: resident sessions impl (261031)
goal: implement "상시" items #1-#8 of plan/interactive-child-resident-sessions.md (:10,:39-42,:45-47,:90); default behavior unchanged (--resident off)
mapping: #1 G3 config -> --resident preset flag | #2 S-fixed 8 slots, lazy spare, 270s idle -> preset fixed + Lazy slots + sweepParked | #3 S-min 2 base / cap 6 code-enforced -> preset min + cap | #4 A/B pair -> NOT impl (needs live measurement runs, cost); stream-json kept | #5 main opus conflict -> --resident-main-model default haiku (CLAUDE.md:33, DECISIONS:40); opus only explicit | #6 cap wiring -> admit() before spawn, LRU-evict parked instead of drop | #7 role key -> childKey(cwd,model,role) | #8 AC4 -> tests for cap + no collision/overwrite
keepalive: plan says off; not implemented (no flag)
stage A (children.go, switchchild.go, dispatch.go):
- childKey(cwd, model, role string) string; role=="" -> identical to old output; update all callers; parkOld uses old.Role
- Park overwrite: only same (cwd,model,role) key retires the existing child
- childSet: Len() int (if missing), EvictLRU() (*liveChild, bool) = remove parked child with oldest LastAPI
- (r *Repl) admit() error: called before every newSessionFn spawn; cap = r.a.MaxWorkers or DefaultMaxWorkers if 0; count = parked + active(1 if present); while count >= cap: EvictLRU + retire; if nothing to evict -> error using checkWorkerCap message
stage B (new resident.go, repl.go flags, idlesweep.go, children.go):
- type residentSlot struct{ Role, Model string; Lazy bool }
- func residentPreset(name, mainModel string) ([]residentSlot, int, error): ""/off -> nil,0; fixed -> eager main(mainModel), tui, coder, prompter (haiku) + lazy coder-spare-1..4 (haiku), cap 8; min -> eager main(mainModel), coder (haiku) + lazy worker-2..4 (haiku), opus-1 (opus), cap 6; other -> error
- flags: --resident (off|min|fixed, env CHAUDE_RESIDENT), --resident-main-model (default haiku); preset cap overrides MaxWorkers unless --max-workers/CHAUDE_MAX_WORKERS set explicitly
- (r *Repl) spawnResident() error: after start() ok, spawn eager non-main slots via newSessionFn, set Role, Park under role key
- (r *Repl) takeOrSpawnSlot(role string) (*liveChild, error): take parked by role or lazy-spawn via admit()
- liveChild.Lazy bool; residentLazyIdle = 270*time.Second; (r *Repl) sweepParked(now) retires Lazy parked children idle > 270s; call where sweepIdle is called
tests: internal/harness/resident_test.go with newSessionFn stub: fixed spawns 3 parked + main; min cap 6; same model diff role both parked; LRU evict at cap; lazy idle 270s retired; off preset -> no extra spawns
must NOT change: core.go args (stream-json), CLAUDE.md, notes/DECISIONS.md, .claude/*, existing test expectations
acceptance: go build ./... ; go vet ./... ; go test ./internal/harness/... all PASS
