> DEPRECATED (2026-10-31): 참조 0건, 원본 계획

# plan: multiproc finish + v0.8.0 release
date: 261029
status: plan only. no source edited.
sources: 261029-plan-multiproc-impl.md, 261029-plan-release-v080.md, 261029-plan-multiproc-rebind.md (rev2, §1-§2), 261029-plan-parallel-load.md (not used for code), notes/DECISIONS.md
version: 0.8.0 (user: "v0.8.0으로 릴리즈"). impl/rebind docs say 0.7.3 -> replaced by 0.8.0 [해석].

## 0. state
- done (per release-v080 plan, re-verify in U-pre): U1a clients.go registry, U1b isWarm, U1c idleDeadline, U2a withCacheTTL, U2b observedTTL, U3a parseCacheTTL, U4a childKey, U4b childSet/CloseAll, U6a pickChild (dispatch.go).
- partial: U3b. flags `--cache-ttl`, `--idle-rotate`, `--clients-file` defined (repl.go:902-904). NOT verified: Args / SessionOpts.CacheTTL pass-through, registry/childSet construction.
- not done: U5 touchMain, U6b classifier switchChild, U7 idleDue, U8 sweepIdle/CloseAll.
- release: U0a (uncommitted tree: classifier.go, conductor_test.go, repl.go, tui.go, ratestatus*.go, LEDGER, metrics, untracked new files) NOT committed. version still 0.7.2 at cmd/chaude/main.go:16. CHANGELOG.md top is 0.6.6.
- deferred to later (not in this plan's 5 units, see §6 doubts): U8b (reconcile orphan check, /clear CloseAll), U9-budget ($3 ledger, budget.go), U5c DesignPath, U10a DECISIONS auto default (approval needed), U10b DECISIONS text.

## 1. gates (must hold before any unit touching repl.go / classifier.go / tui.go)
- G1 [승인필요] U0a commit approval from user. Until approved: only pure units (clients.go, children.go, core.go, dispatch.go) may run. repl.go/classifier.go/tui.go units WAIT.
- G2 [승인필요] version name confirm: 0.8.0 (user) vs 0.7.3 (impl doc). Plan uses 0.8.0.
- G3 default cache TTL stays 1h (DECISIONS:23) until user approves auto (U10a). Not in this plan's units.
- G4 live gate (TestClassifierLive, CHAUDE_LIVE=1, DECISIONS:33 convention) run only with user OK for cost. Live cost cap per task $3 (user rule), first a $0.01-class smoke check.

## 2. invariants (do NOT change)
- rotate() handoff/verify semantics; drain path (DECISIONS:18); haikupool selection (rr.Worker priority, not counted in cap); cleanEnv signature and ANTHROPIC_API_KEY removal; Session.Close semantics; no Setpgid; r.agents.interruptRunning behavior.
- same-model haiku worker switch: when target == r.a.Model and rr.Worker != "", keep existing switchSession(model, worker) park-resume path (classifier.go:176-179 body unchanged). Live-child path is used only when model differs.
- r.sess is the only active-slot field. No second owner of a child: parked child is never also in r.parked (no double ownership).
- SIGHUP/SIGKILL: no PID kill logic added in this plan.

## 3. units

Legend: H = Haiku-executable (mechanical, exact spec given). S = needs design/sonnet-analyst (coupled repl.go/classifier.go/tui.go). Each unit acceptance = `go build ./... && go vet ./internal/harness/ && go test ./internal/harness/... -count=1` plus the named tests. Haiku fails 2x -> escalate to sonnet-analyst.

### U-pre (H) verify baseline
- files: none edited.
- do: `go test ./internal/harness/... -count=1 -v` and record PASS per test name for U1a-U4b, U6a, U3a. Grep repl.go for `cacheTTL|idleRotate|clientsFile` in Main (repl.go ~902-904) and the Args struct / SessionOpts construction (grep `SessionOpts{`, `Args{`).
- output: notes/runs/261029-check-multiproc-baseline.md (terse key: value).
- acceptance: `go build ./... && go vet ./... && go test ./... -count=1` exit 0.

### U3b (S-light; wiring) finish Args/registry wiring
- files: internal/harness/repl.go (Main, Args struct, start() SessionOpts construction ~repl.go:177), internal/harness/core.go (SessionOpts.CacheTTL field only if missing).
- functions: `Main` (flag parse), `Args` fields `CacheTTL string`, `IdleRotate time.Duration`, `ClientsFile string`; `(r *Repl) start()` sets `SessionOpts.CacheTTL = parseCacheTTL(a.CacheTTL)` result; construct `r.reg = newClientRegistry(ClientsFile)` when ClientsFile != "", `r.children = &childSet{}`; when ClientsFile == "" both nil (route() falls back to old code).
- default: --cache-ttl default "1h"; --idle-rotate default 3599s; 0 disables.
- depends: U2a, U3a (done).
- tests: TestCacheTTLFlagWiring (new, in cachettl_test.go): Args{CacheTTL:"5m"} -> SessionOpts.CacheTTL "5m" via newSessionFn seam; TestRegistryNilWhenNoFile.
- acceptance: `go build ./... && go vet ./internal/harness/ && go test ./internal/harness/... -count=1 -run 'CacheTTL|Registry|Wiring'` PASS; then full `go test ./internal/harness/... -count=1`.
- type: H for the mechanical field/flag passthrough (after U-pre names the exact lines); S only if U-pre finds the struct already diverges.

### U5 (S) touchMain + parked-event plumbing (split per rebind rev2: U5a, U5b, U5d; U5c DesignPath deferred)
- U5a files: repl.go. functions: `(r *Repl) touchMain(ev Event, now time.Time)`; liveChild gets `mu sync.Mutex`, `LastAPI time.Time`, `parked atomic.Bool`; start() allocates `*liveChild`, idle closure captures it; `idleEvent` branch for parked child (renders nothing, buffers main reply, queues notice "[parked <model> background reply] <300 chars>", can_use_tool auto-deny "denied: session parked (another model is active)").
  - rule: touchMain runs only for isMainEvent (not subagent) and message_start -> LastAPI; assistant usage -> r.ttlSeen = observedTTL(usage, r.ttlSeen).
  - rule: `s == r.sess` comparisons forbidden; use `c.parked.Load()`.
  - rule: API call never made (LastAPI zero) -> Close instead of Park (done in U6b).
  - tests: TestTouchMainOnly, TestIdleEventFromParkedChild, TestIdleEventParkedNoRender, TestIdleEventParkedPermDenied. `go test -race ./internal/harness/ -run 'TouchMain|IdleEventParked|IdleEventFromParked' -count=1`.
  - acceptance: build, vet, race test above, full harness test.
  - type: S (concurrency; mutex and atomic placement).
- U5b files: repl.go. functions: `(r *Repl) unbindActive() *liveChild`, `(r *Repl) bindChild(c *liveChild)`.
  - unbindActive: copy per-child fields (sess, tracker, Model, lastCtx, cat, LastAPI, TurnFrom, pendingSnapshot, lastRotFail, ttlSeen, idle accumulator) into c; c.parked.Store(true) FIRST; c.TurnFrom = len(r.tr.turns); r.sess = nil; keep r.tracker non-nil (tui.go:312/329/378/983 read it per tick). No Close.
  - bindChild: reverse copy; r.a.Model = c.Model; c.parked.Store(false) LAST. If len(r.tr.turns) > c.TurnFrom, append deterministic delta to r.pendingSnapshot: `## While this session was parked` + turns[TurnFrom:] + files added since; cap haiku 3000 chars, opus/sonnet 6000 chars (DECISIONS overview cap). No new turns -> pendingSnapshot untouched.
  - tui.go /status (~tui.go:890) nil guard for r.sess == nil (pattern localcmd.go:36).
  - tests: TestBindUnbindRoundTrip (Close count 0), TestBindAddsParkedDelta, TestBindNoDeltaKeepsEmpty, TestTrackerNeverNil.
  - acceptance: build, vet, `go test ./internal/harness/ -run 'BindUnbind|BindAdds|BindNoDelta|TrackerNeverNil' -count=1`, full harness.
  - type: S.
- U5d files: repl.go. function: `(r *Repl) makeSnapshot(reason string) (snap string, ok bool)` extracted from rotate (handoff turn + verify + withReco; no close, no start). rotate calls it; behavior unchanged. Fixes empty-session rotate panic.
  - tests: existing rotate/handoff tests unmodified and PASS (regression gate).
  - acceptance: `go test ./internal/harness/ -count=1` all existing rotate tests PASS, plus build/vet.
  - type: S (must not alter handoff semantics). Haiku may do the pure extraction only after S defines the signature.

### U6b (S) classifier model-switch via live children
- files: classifier.go (route, switchSession call sites ~:181-198), new seam `var classifyFn = classify`.
- function: `(r *Repl) switchChild(target, worker string) error` (5 steps):
  1. target == r.a.Model: if rr.Worker != "" -> existing switchSession(target, worker) (same-model haiku park-resume kept); else no-op.
  2. target != r.a.Model: if history exists and warm target exists -> go to 3. Else snapshot = makeSnapshot(reason) while old child still active; old := unbindActive(); if LastAPI zero -> Close old; else childSet.Park(old) (Park never writes r.parked).
  3. target == haiku && rr.Worker != "": if childSet has haiku key -> Take; close or drain (drain if agents running); then switchSession body with Close excluded.
  4. else pickChild (U6a): actUseLive -> Take + bindChild (dead child -> actStartNew + snapshot); actResume -> r.resumeID = row.SessionID; r.start(); actStartNew -> r.parked[model] resume if present, else r.start() and r.pendingSnapshot = snap.
  5. start failure: restore old child via Take + bindChild; registry row back to live; r.a.Model restored; print error.
- rules: a childSet key with !isWarm -> Take, then drain/close, then registry decides. SessionID present in childSet or draining is never resumed.
- route(): registry/childSet nil -> old code unchanged. Park-resume check moved before haiku worker branch (classifier.go:175-179) so live path is reachable.
- tests: TestRouteKeepsPreviousChildLive (haiku->sonnet->haiku: newSessionFn 2x, third reuses, Close 0), TestRouteToHaikuParksLive (opus->haiku Close 0), TestRouteBackUsesLiveNotParkedMap, TestRouteWorkerSwitchClosesParkedHaiku, TestNoResumeOfLiveSessionID, TestRouteDeadChildFallsBack, TestRouteFallsBackWithoutRegistry (existing conductor_test regression 0), TestRouteStartFailRollsBack, TestRouteSameModelWorkerUsesOldPath (Close 1, childSet unchanged).
- acceptance: build, vet, `go test ./internal/harness/ -run 'Route|NoResume' -count=1`, full harness test.
- type: S. Reviewer: sonnet-reviewer; if review fails twice -> opus-architect (per rebind rev2 §2).
- depends: U5b, U5d, U6a.

### U7 (H for pure part, S for wiring) idleDue
- files: repl.go (method), clients.go (helper reuse).
- function: `(r *Repl) idleDue(now time.Time) bool`: deadline := idleDeadline(r.idleRotate, r.ttl) (U1c); deadline <= 0 -> false; no turn since last rotation -> false; now - r.lastAPI >= deadline; same lastAPI already fired -> false (store r.idleFiredFor time.Time).
- helper: `expiredZero(deadline) bool` pure in clients.go (deadline<=0 = never expires).
- tests: TestIdleDue (3598s false, 3599s true, second call same lastAPI false, rotate-then-no-turn false), TestIdleDueDisabled, TestExpiredZeroDeadline.
- acceptance: `go test ./internal/harness/ -run 'IdleDue|ExpiredZero' -count=1` plus build, vet, full harness test.
- type: S for the method (repl state); the pure `expiredZero` and its test are H.
- depends: U5a (lastAPI), U1c.

### U8 (S) sweepIdle + CloseAll wiring
- files: tui.go, repl.go.
- functions: `(r *Repl) sweepIdle(now time.Time)` called from spinner.TickMsg handler (tui.go, ~tui.go:374, 1s limit, only when !busy and no permission prompt, per A6):
  - parked child + tracker.running() > 0 -> drain (T2) instead of Close.
  - parked child dead (s.lines closed) -> Take + drop, registry expired "child-exit" (T3).
  - active child with idleDue(now) -> makeSnapshot("idle <N>s") -> pendingSnapshot -> rotate-like close -> registry expired. Done via the same path as user prompt rotation (not directly inside Update loop).
  - parked child with now-LastAPI >= deadline -> Close, expired, no handoff.
- CloseAll: `children.CloseAll()` (parallel, U4b) called from tui.go quit defer (~tui.go:1117, outside `if r.sess != nil`) and repl.go loop() defer (~repl.go:810).
- registry on exit: State=parked, EndedReason="exit", SessionID kept.
- tests: TestSweepIdleClosesParkedOnly, TestSweepDrainsBusyParked, TestQuitCallsCloseAll, TestPlainLoopCallsCloseAll.
- acceptance: build, vet, `go test ./internal/harness/ -run 'Sweep|CloseAll|Quit' -count=1`, full harness test. Manual: `chaude --idle-rotate 5s` -> 5s later handoff + close, clients.json State=expired, next prompt carries snapshot.
- type: S. Non-TTY REPL has no timer (document).
- depends: U7, U6b.

### U-rel-1 (H) version + CHANGELOG (after gates)
- files: cmd/chaude/main.go:16 `var version = "0.8.0"` (from "0.7.2"); CHANGELOG.md new top section `## 0.8.0`.
- CHANGELOG text: 모델별 세션 유지(한 번에 하나씩 작업), 유휴 3599초 핸드오프 후 종료, 캐시 TTL 관측(5m 감지 시 290초), size tag 기본 large, 5h 사용량 상태줄. Forbidden words: "multi-client", "동시 실행". Do not fill 0.6.7-0.7.2 gap.
- acceptance: `go build ./... && go vet ./... && go run ./cmd/chaude --version` prints 0.8.0.
- type: H.
- depends: all units PASS, U0a commit done.

### U0a (H, commit only after G1 approval) split commit of pending tree
- order: (1) code commit: classifier.go, conductor_test.go, repl.go, tui.go, ratestatus.go, ratestatus_test.go; message `classifier: size tag defaults to large; status line: 5h rate usage`; version not bumped here. (2) metrics commit: notes/LEDGER.md + notes/metrics/2026-10-29.jsonl, `metrics: 2026-10-29`. (3) notes (notes/runs/*.md, notes/research/*) as docs commit or left untracked.
- note: U0a code touches repl.go/tui.go/classifier.go, so it must be committed BEFORE U3b-U8 to keep the 0.8.0 diff clean; that is the reason G1 gates everything below. Ordering: approval -> U0a -> U-pre -> rest.
- acceptance: `git status` shows only the 0.8.0 files; build/vet/test PASS before commit.
- type: H for staging; decision is user's.

### U-rel-2 (S) release review and commit
- sonnet-reviewer once on 0.8.0 diff (post-U0a only). Fix findings, re-run acceptance.
- commit: `0.8.0: per-model live child processes, idle rotate at 3599s, cache TTL observe` + final line `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`. Note: session attribution rule names Haiku 5.5; this matches the given instruction.
- tag: `git tag v0.8.0` ONLY on user request. No push.
- acceptance: `go test ./... -count=1` all ok (no `(cached)`), `go vet ./...`, `go build ./...`.

## 4. ordering
U-pre -> [G1 approval] -> U0a -> U1-U4 already done -> U3b -> U5a -> U5b -> U5d -> U6b -> U7 -> U8 -> U-rel-1 -> U-rel-2.
parallel allowed: U-pre with U0a staging prep; U7 pure helper with U5a.

## 5. acceptance (all units)
- every unit: `go build ./...` exit 0; `go vet ./...` exit 0; `go test ./internal/harness/... -count=1` PASS (no cache). U5a also `go test -race ./internal/harness/ -count=1`.
- release: `go test ./... -count=1` all ok; `go run ./cmd/chaude --version` = 0.8.0; `git tag` v0.8.0 only after user request.
- manual (after U8): `chaude --idle-rotate 5s`; `ps` orphan 0 after exit.
- live tests: $3 cap per task; smoke first; skip if cap mechanism unverified.

## [haiku] steps
Order (serial unless noted). Haiku runs only the H units; S units go to sonnet-analyst.
1. U-pre (H): baseline test + name-to-PASS record.
2. U3b (H part): passthrough of CacheTTL/IdleRotate/ClientsFile wiring, only after U-pre confirms exact lines. Tests: TestCacheTTLFlagWiring.
3. U7-pure (H): `expiredZero` in clients.go + TestExpiredZeroDeadline.
4. U-rel-1 (H): only after all gates and U-rel-2 PASS: version 0.8.0 + CHANGELOG `## 0.8.0`.
5. U0a staging (H): only after G1 approval.
S units (not Haiku): U3b wiring decision (if divergence), U5a, U5b, U5d, U6b, U7 method, U8, U-rel-2.

## 6. doubts / out of this plan
- constraint scope: only 5 units requested (U3b, U5, U6b, U7, U8). Rebind rev2 also defines U8b (reconcile orphan check T6, /clear CloseAll T7), U5c (DesignPath), and U9-budget ($3 ledger, budget.go, --max-budget-usd second layer). Not included; decide whether they block 0.8.0 or move to 0.8.1.
- version: user says 0.8.0, impl docs say 0.7.3. Plan uses 0.8.0.
- U5 split: user lists "U5 touchMain"; rev2 splits it into U5a/U5b/U5d. Plan follows the split.
- U10a (DECISIONS auto TTL default) needs approval; plan keeps 1h default. CHANGELOG TTL wording then "캐시 TTL 관측" not "auto".
- live gate (TestClassifierLive) cost approval pending.
- main.go version / --version check not yet run in this session.
- TTL 5m observed-rate; A1-A12 assumptions in impl doc §6 remain unverified; A3 (live child rebinding) is the riskiest, covered by TestRouteKeepsPreviousChildLive plus manual 3-model round trip.
- untracked notes/* files are not part of the release commit unless user says so.
