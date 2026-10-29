# plan: multiproc finish + v0.8.0 release
date: 261029
status: plan only. no source edited. rev2: review 261029-plan-multiproc-finish-review.md merged (A1-A8, B1-B5, C1-C5, D1-D4, E1-E5). original: notes/archive/261029-plan-multiproc-finish.orig.md
sources: 261029-plan-multiproc-impl.md, 261029-plan-release-v080.md, 261029-plan-multiproc-rebind.md (rev2, §1-§2), 261029-plan-parallel-load.md (not used for code), notes/DECISIONS.md
version: 0.8.0 (user: "v0.8.0으로 릴리즈"). impl/rebind docs say 0.7.3 -> replaced by 0.8.0.
decisions applied (user, final): D1 newChildSet() constructor, field names from children.go. D2 release runs last (after U8). D3 0.8.0 scope = U3b, U5, U6b, U7, U8 only. D4 trailer `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`. D5 name/signature fixes A1-A8 applied.

## 0. state
- done (re-verify in U-pre): U1a clients.go registry, U1b isWarm, U1c idleDeadline, U2a withCacheTTL (core.go:41), U2b observedTTL (core.go:78), U3a parseCacheTTL (repl.go:869), U4a childKey (children.go:14), U4b childSet/CloseAll (children.go), U6a pickChild (dispatch.go:61).
- partial: U3b. flags `--cache-ttl`, `--idle-rotate`, `--clients-file` defined (repl.go:902-904). CacheTTL passthrough already wired: repl.go:181 `SessionOpts{... CacheTTL: r.a.CacheTTL}`; repl.go:912 validates via parseCacheTTL. Missing: Repl fields reg, children, idleRotate, ttl, lastAPI, ttlSeen (grep empty).
- not done: U5a/U5b/U5d touchMain + bind/unbind + makeSnapshot, U6b classifier switchChild, U7a/U7b idleDue, U8 sweepIdle/CloseAll.
- release: U0a (uncommitted tree: classifier.go, conductor_test.go, repl.go, tui.go, ratestatus*.go, LEDGER, metrics, untracked new files) NOT committed. version still 0.7.2 at cmd/chaude/main.go:16. CHANGELOG.md top is 0.6.6.
- 0.8.x follow-up (NOT in 0.8.0): U8b (reconcile orphan check T6, /clear CloseAll T7), U5c (DesignPath), U9-budget ($3 ledger, budget.go, --max-budget-usd), U10a (DECISIONS auto TTL default, approval needed), U10b (DECISIONS text).

## 1. gates (must hold before any unit touching repl.go / classifier.go / tui.go)
- G1 [승인필요] U0a commit approval from user. Until approved: only pure units (clients.go, children.go, core.go, dispatch.go) may run. repl.go/classifier.go/tui.go units WAIT.
- G2 version name: 0.8.0 (user). impl doc 0.7.3 is superseded. Plan uses 0.8.0.
- G3 default cache TTL stays 1h (DECISIONS:23) until user approves auto (U10a). Not in 0.8.0.
- G4 live gate (TestClassifierLive, CHAUDE_LIVE=1, DECISIONS:33 convention) run only with user OK for cost. Live cost cap per task $3. First a $0.01-class smoke check.

## 2. invariants (do NOT change)
- rotate() handoff/verify semantics; drain path (DECISIONS:18); haikupool selection (rr.Worker priority, not counted in cap); cleanEnv signature and ANTHROPIC_API_KEY removal; Session.Close semantics; no Setpgid; r.agents.interruptRunning behavior.
- same-model haiku worker switch: when target == r.a.Model and rr.Worker != "", keep existing switchSession(model, resume) park-resume path (classifier.go:175-179 body unchanged). Live-child path is used only when model differs.
- r.sess is the only active-slot field. No second owner of a child: a parked child is never also in r.parked (no double ownership). r.parked is only written by the actStartNew resume-map path, never by Park.
- single active clock: r.lastAPI is the only clock for the active child. unbindActive copies it to c.LastAPI; bindChild copies it back. No second live clock.
- SIGHUP/SIGKILL: no PID kill logic added in this plan.

## 3. authoritative names (from code; plan text must use these)
- childKey(cwd, model string) string — children.go:14.
- type closer interface{ Close() } — children.go:30. *Session satisfies it.
- liveChild struct — children.go:33: `Sess closer; Tracker *agentTracker; Model string; Role string; LastAPI time.Time; LastCtx int; Files []string; TurnFrom int`. Sess is `closer`, NOT *Session.
- childSet struct — children.go:45: `mu sync.Mutex; m map[string]*liveChild`.
- constructor: `newChildSet() *childSet` — children.go:50. Only way to make a childSet (a struct literal leaves m nil; Park then panics).
- `(s *childSet) Park(key string, c *liveChild)` — children.go:55. Replaces and closes any existing child under key (only if Sess differs). Does not touch r.parked.
- `(s *childSet) Take(key string) (*liveChild, bool)` — children.go:66.
- `(s *childSet) Expired(now time.Time, deadline time.Duration) []string` — children.go:78. Keys only; caller Takes and closes.
- `(s *childSet) CloseAll()` — children.go:93. Parallel.
- pickChild(set *childSet, reg *clientRegistry, key, cwd, model string, limit int, now time.Time, deadline time.Duration) childAction — dispatch.go:61. Values: actStartNew, actUseLive, actResume.
- newClientRegistry(path string) *clientRegistry — clients.go:43. Methods: Get(cwd, model) (clientRec, bool); Upsert(rec); MarkEnded(cwd, model, reason string, now time.Time); reconcile(now, deadline).
- isWarm(rec clientRec, limit int, now time.Time, deadline time.Duration) bool — clients.go:135.
- idleDeadline(idleRotate time.Duration, ttl string) time.Duration — clients.go:145. ttl is a string.
- defaultClientsFile() string — clients.go:48.
- observedTTL(usage map[string]any, prev string) string — core.go:78.
- withCacheTTL(env []string, ttl string) []string — core.go:41.
- parseCacheTTL(s string) (string, error) — repl.go:869.
- Repl.start() error — repl.go:180. Repl.rotate(reason string) — repl.go:575.
- classify(line, cur, lastUser string, cands []haikuWorker) (rr routeResult, cost float64, usage map[string]any, err error) — classifier.go:178.
- (r *Repl) route(line string) string — classifier.go:209.
- (r *Repl) switchSession(model, resume string) — classifier.go:269. `resume` is a SESSION ID. rr.Worker is passed as that session ID.
- existing Repl fields: pendingSnapshot string (repl.go:141), r.tr.turns, r.tracker, r.sess, r.resumeID, r.parked (map), r.lastRotFail.
- NEW Repl fields (U3b/U5a): reg *clientRegistry; children *childSet; idleRotate time.Duration; ttl string; lastAPI time.Time; ttlSeen string; idleFiredFor time.Time.
- Args fields exist: CacheTTL string, IdleRotate time.Duration, ClientsFile string (repl.go:62-64).
- Tests: TestCacheTTLFlagWiring NOT present in cachettl_test.go? U-pre checks before adding; add only if missing.

## 4. units

Legend: H = Haiku-executable (mechanical, exact spec given). S = needs design/sonnet-analyst (coupled repl.go/classifier.go/tui.go). Each unit acceptance = `go build ./... && go vet ./internal/harness/ && go test ./internal/harness/... -count=1` plus the named tests. Haiku fails 2x -> escalate to sonnet-analyst.

### U-pre (H) verify baseline
- files: none edited.
- do: `go test ./internal/harness/... -count=1 -v`; record PASS per test name for U1a-U4b, U6a, U3a. Record `git diff --stat | shasum` of the dirty tree (B4: baseline is only valid for this tree). Grep repl.go/children.go/clients.go/classifier.go for each name in §3 and mark present/absent. Check `grep -n "TestCacheTTLFlagWiring\|TestRegistryNilWhenNoFile" internal/harness/*_test.go`. Check liveChild's per-child fields (cat, pendingSnapshot, lastRotFail, ttlSeen) against Repl fields. Check TestWheelScrollHoldsPosition exists in agentview_test.go.
- output: notes/runs/261029-check-multiproc-baseline.md (terse key: value).
- acceptance: `go build ./... && go vet ./... && go test ./... -count=1` exit 0.

### U0a (H, staging; only after G1 approval) split commit of pending tree
- order: (1) code commit: classifier.go, conductor_test.go, repl.go, tui.go, ratestatus.go, ratestatus_test.go; message `classifier: size tag defaults to large; status line: 5h rate usage`; version not bumped here. (2) metrics commit: notes/LEDGER.md + notes/metrics/2026-10-29.jsonl, `metrics: 2026-10-29`. (3) notes (notes/runs/*.md, notes/research/*) docs commit or left untracked.
- must be committed BEFORE U3b..U8 so the 0.8.0 diff stays clean.
- acceptance: `git status` shows only the 0.8.0 files; build/vet/test PASS before commit.
- type: H for staging; the decision is the user's.

### U3b (H; wiring only) Repl fields and registry/childSet construction
- files: internal/harness/repl.go (Repl struct, Main, start()).
- CacheTTL passthrough: ALREADY WIRED (repl.go:181, repl.go:912). Do not change SessionOpts.
- do: add Repl fields `reg`, `children`, `idleRotate`, `ttl`, `lastAPI`, `ttlSeen`, `idleFiredFor` (§3). In Main/start(): `r.idleRotate = a.IdleRotate`; `r.ttl = a.CacheTTL`; if `a.ClientsFile != ""` then `r.reg = newClientRegistry(a.ClientsFile)` and `r.children = newChildSet()`; else both stay nil (route() uses old code).
- constructor: `newChildSet()` only (A2). Never a struct literal.
- defaults: --cache-ttl "1h"; --idle-rotate 3599s; 0 disables (repl.go:902-904, unchanged).
- note (B1): this creates r.children before liveChild gains fields in U5a. Fine: construction does not depend on liveChild fields.
- tests: TestCacheTTLFlagWiring (Args{CacheTTL:"5m"} -> SessionOpts.CacheTTL "5m" via newSessionFn seam; add only if U-pre shows missing), TestRegistryNilWhenNoFile (new).
- acceptance: `go build ./... && go vet ./internal/harness/ && go test ./internal/harness/... -count=1 -run 'CacheTTL|Registry|Wiring'` PASS; then full `go test ./internal/harness/... -count=1`.
- type: H. S only if U-pre finds the struct already diverges.
- depends: U-pre, U2a, U3a (done).

### U5 (S) touchMain + parked-event plumbing (split: U5a, U5b, U5d; U5c DesignPath is 0.8.x follow-up)

#### U5a (S) liveChild fields, touchMain, parked idle branch
- files: internal/harness/children.go (liveChild), internal/harness/session file (Dead method), repl.go.
- liveChild additions (names keep existing fields as §3): `mu sync.Mutex` guards LastAPI; `parked atomic.Bool`. Sess stays `closer` (A3). Dead-check: *Session gets `Dead() bool` (reads closed s.lines); code asserts `c.Sess.(interface{ Dead() bool })`. No change to closer.
- `(r *Repl) touchMain(ev Event, now time.Time)`: runs only for isMainEvent (not subagent). message_start -> r.lastAPI = now (single clock, B2). assistant usage -> r.ttlSeen = observedTTL(usage, r.ttlSeen).
- rule: `s == r.sess` comparisons forbidden; use `c.parked.Load()`.
- idle branch for parked child: renders nothing; buffers main reply; queues notice "[parked <model> background reply] <300 chars>" appended at the NEXT user prompt, not rendered immediately (E2); can_use_tool auto-deny "denied: session parked (another model is active)".
- rule: API call never made (LastAPI zero) -> Close instead of Park (done in U6b).
- tests: TestTouchMainOnly, TestIdleEventFromParkedChild, TestIdleEventParkedNoRender, TestIdleEventParkedPermDenied, TestParkedReplyNoUnseen (m.unseen not set, E2).
- acceptance: build, vet, `go test -race ./internal/harness/ -run 'TouchMain|IdleEventParked|IdleEventFromParked|ParkedReplyNoUnseen' -count=1`, full harness.
- type: S (mutex and atomic placement).

#### U5b (S) unbindActive / bindChild
- files: repl.go, tui.go (/status handler).
- functions: `(r *Repl) unbindActive() *liveChild`, `(r *Repl) bindChild(c *liveChild)`.
- field mapping (A3): r.sess -> c.Sess (store as closer); bind does `r.sess = c.Sess.(*Session)`. r.tracker -> c.Tracker. r.a.Model -> c.Model. r.lastAPI -> c.LastAPI (under c.mu). r.lastCtx -> c.LastCtx. r.tr.turns cursor -> c.TurnFrom. files -> c.Files. Fields with no liveChild home today (pendingSnapshot, lastRotFail, ttlSeen, cat, idle accumulator) are added to liveChild with the same names; U-pre confirms each Repl name.
- unbindActive: c.parked.Store(true) FIRST; c.TurnFrom = len(r.tr.turns); r.sess = nil; keep r.tracker non-nil (tui.go TickMsg and /status read it). No Close.
- bindChild: reverse copy; r.a.Model = c.Model; c.parked.Store(false) LAST. If len(r.tr.turns) > c.TurnFrom: append deterministic delta to r.pendingSnapshot: `## While this session was parked` + turns[TurnFrom:] + files added since; cap haiku 3000 chars, opus/sonnet 6000 chars (DECISIONS overview cap). No new turns -> pendingSnapshot untouched.
- tui.go /status handler (function that reads `m.r.sess.SessionID`, A7): nil guard for r.sess == nil (pattern localcmd.go:36). TickMsg reads m.r.tracker.running() (A7); tracker is never nil so no guard needed there.
- E3: model switch while m.held: switch banner goes through r.p (refresh path); offset kept.
- tests: TestBindUnbindRoundTrip (Close count 0), TestBindAddsParkedDelta, TestBindNoDeltaKeepsEmpty, TestTrackerNeverNil, TestUnbindParkedRace (C1: idle callback racing Park flag; -race).
- acceptance: build, vet, `go test -race ./internal/harness/ -run 'BindUnbind|BindAdds|BindNoDelta|TrackerNeverNil|UnbindParkedRace' -count=1`; `go test -count=1 -race ./internal/harness -run 'ScrollBottom|WheelScroll'` (E5); full harness.
- type: S.
- depends: U5a.

#### U5d (S) makeSnapshot extraction (3-state)
- files: repl.go.
- function: `(r *Repl) makeSnapshot(reason string) (snap string, st snapState)`; `snapState` consts `snapOK`, `snapHeld`, `snapInvalid`. Extracted from rotate: handoff turn + verify + withReco; no close, no start.
- held (A8): the lastRotFail hold (repl.go:600-611) is checked in makeSnapshot and returns snapHeld; rotate performs the held side effects (logRotate, r.p) exactly as today. snapInvalid = carried (repl.go:606 zero-turn guard).
- A8: the claim "fixes empty-session rotate panic" is DROPPED (no panic path shown; repl.go:606 already guards).
- rotate calls it; behavior unchanged.
- tests: existing rotate/handoff tests unmodified and PASS (regression gate); TestMakeSnapshotStates (new; three states).
- acceptance: `go test ./internal/harness/ -count=1` all existing rotate tests PASS, plus build/vet.
- type: S (must not alter handoff semantics). Haiku may do the pure extraction only after S defines the signature. Not assigned to Haiku in §7.
- depends: U5a.

### U6b (S) classifier model-switch via live children
- files: classifier.go (route, switchSession call sites ~:181-198, seam at classify call ~:218).
- step 0 (A6): add `var classifyFn = classify` (signature identical to classify at classifier.go:178); route calls classifyFn. Tests use a fake classifyFn; no real `claude` exec (cost).
- function: `(r *Repl) switchChild(target, resumeID string) error` (A5: param named resumeID; it is a session id, not a worker name).
- steps:
  1. target == r.a.Model: if rr.Worker != "" -> existing switchSession(target, rr.Worker) (same-model haiku park-resume kept, body unchanged); else no-op.
  2. target != r.a.Model: if history exists and warm target exists -> go to 4. Else: snap, st := makeSnapshot(reason) while the old child is still active; old := unbindActive(); if old.LastAPI is zero -> old.Sess.Close(); else park old: key := childKey(r.a.Cwd, old.Model); if an entry exists under key with running agents, drain it first (Take, drain); else Take and Close it; then `r.children.Park(key, old)`. Park never writes r.parked (A4, C2).
  3. target == haiku && rr.Worker != "": if children has haiku key -> Take; close or drain (drain if agents running); then switchSession body with Close excluded (the old session is already unbound).
  4. else pickChild(r.children, r.reg, childKey(r.a.Cwd, target), r.a.Cwd, target, limit, now, deadline) (A-pick): actUseLive -> Take(key) + bindChild(c) (dead child -> actStartNew + snapshot); actResume -> r.resumeID = row.SessionID; r.start(); actStartNew -> if r.parked[target] resume is present, use it; else r.start() and r.pendingSnapshot = snap.
  5. start failure: restore old via Take(key) + bindChild(old); registry row back to live (reg.Upsert); r.a.Model restored; error printed through r.p (E4), NOT fmt.Fprintln(os.Stderr) (alt-screen corruption).
- rules: a childSet key with !isWarm -> Take, then drain/close, then registry decides. A SessionID present in childSet or draining is never resumed.
- route(): reg/children nil -> old code unchanged. Park-resume check moved before the haiku worker branch (classifier.go:175-179) so the live path is reachable.
- tests: TestParkReplaceClosesOnce (same key twice -> older child closed exactly once; C2), TestRouteKeepsPreviousChildLive (haiku->sonnet->haiku: newSessionFn 2x, third reuses, Close 0), TestRouteToHaikuParksLive (opus->haiku Close 0), TestRouteBackUsesLiveNotParkedMap, TestRouteWorkerSwitchClosesParkedHaiku, TestNoResumeOfLiveSessionID, TestRouteDeadChildFallsBack, TestRouteFallsBackWithoutRegistry (existing conductor_test regression 0), TestRouteStartFailRollsBack, TestRouteStartFailPrintsViaRepl (E4), TestRouteSameModelWorkerUsesOldPath (Close 1, childSet unchanged).
- acceptance: build, vet, `go test ./internal/harness/ -run 'Route|NoResume|ParkReplace' -count=1`, full harness test.
- type: S. Reviewer: sonnet-reviewer; if review fails twice -> opus-architect.
- depends: U5b, U5d, U6a.

### U7 idleDue (split)

#### U7a (H) expiredZero pure helper
- files: internal/harness/clients.go.
- function: `expiredZero(deadline time.Duration) bool` — deadline <= 0 means never expires -> returns false. Name is for the pure helper only.
- tests: TestExpiredZeroDeadline (0 and negative -> false; positive -> true when elapsed >= deadline is NOT this function; keep it to the deadline<=0 rule only).
- acceptance: `go test ./internal/harness/ -run 'ExpiredZero' -count=1` plus build, vet.
- type: H. Can run in parallel with U5a (different file).
- depends: none.

#### U7b (S) idleDue method
- files: repl.go.
- function: `(r *Repl) idleDue(now time.Time) bool`: deadline := idleDeadline(r.idleRotate, r.ttl); if expiredZero(deadline) -> false; no turn since last rotation -> false; now.Sub(r.lastAPI) >= deadline; if r.idleFiredFor equals r.lastAPI -> false, else set r.idleFiredFor = r.lastAPI on fire (fire once per lastAPI).
- single clock: r.lastAPI (B2). Not a second clock.
- tests: TestIdleDue (3598s false, 3599s true, second call same lastAPI false, rotate-then-no-turn false), TestIdleDueDisabled.
- acceptance: `go test ./internal/harness/ -run 'IdleDue' -count=1` plus build, vet, full harness test.
- type: S.
- depends: U5a, U3b, U7a.

### U8 (S) sweepIdle + CloseAll wiring
- files: tui.go, repl.go, (repl.go loop()).
- functions: `(r *Repl) sweepIdle(now time.Time)` called from the spinner TickMsg handler (tui.go, ~1s, only when !busy and no permission prompt; TickMsg passes time.Now() only). Tests C3: sweep does not run while busy or during a permission prompt.
- actions:
  - parked child with its Tracker.running() > 0 -> drain (T2) instead of Close.
  - parked child dead (Sess implements Dead() true) -> Take + drop; registry MarkEnded(cwd, model, "child-exit", now) (T3).
  - active child with idleDue(now) -> makeSnapshot("idle <N>s") -> r.pendingSnapshot -> rotate(reason) (same path as user-prompt rotation; not directly inside Update). rotate keeps an existing pendingSnapshot (repl.go:578-584 snapshot_kept, B3); registry MarkEnded expired.
  - parked child with now - LastAPI >= deadline -> keys from children.Expired(now, deadline); Take each -> Close; registry MarkEnded expired; no handoff.
- notices (E1): every notice (rotate banner, "[parked ...]", expired msg) goes through r.p -> refresh(). Never call m.vp.GotoBottom() from sweep.
- CloseAll (C5): `children.CloseAll()` called from the tui.go quit defer (outside `if r.sess != nil`) and repl.go loop() defer (~repl.go:810). Nil-safe when r.children is nil.
- registry on exit: State=parked, EndedReason="exit", SessionID kept.
- D4: non-TTY REPL has no timer. --help text for --idle-rotate says "TUI only" (not docs only).
- tests: TestSweepIdleClosesParkedOnly, TestSweepDrainsBusyParked, TestSweepSkippedWhileBusy, TestSweepSkippedDuringPermPrompt (C3), TestIdleRotateKeepsPendingSnapshot (B3), TestIdleNoticeKeepsHeldOffset (E1, agentview_test.go next to TestWheelScrollHoldsPosition), TestQuitCallsCloseAll, TestPlainLoopCallsCloseAll, TestQuitCloseAllWithNilSess (C5).
- acceptance: build, vet, `go test ./internal/harness/ -run 'Sweep|CloseAll|Quit|IdleRotate|IdleNotice' -count=1`; `go test -race -count=1 ./internal/harness -run 'ScrollBottom|WheelScroll'` (E5); full harness test. Manual: `chaude --idle-rotate 5s` -> 5s later handoff + close, clients.json State=expired, next prompt carries snapshot.
- type: S.
- depends: U7b, U6b.

### U-rel-1 (H) version + CHANGELOG (after U8 PASS)
- files: cmd/chaude/main.go:16 `var version = "0.8.0"` (from "0.7.2"); CHANGELOG.md new top section `## 0.8.0`.
- D1: before the bump, grep Makefile / goreleaser / ldflags for a pinned version; main.go:15 comment says -ldflags can override.
- CHANGELOG text: 모델별 세션 유지(한 번에 하나씩 작업), 유휴 3599초 핸드오프 후 종료, 캐시 TTL 관측(5m 감지 시 290초), size tag 기본 large, 5h 사용량 상태줄. Forbidden words: "multi-client", "동시 실행". Do not fill the 0.6.7-0.7.2 gap.
- acceptance: `go build ./... && go vet ./... && go run ./cmd/chaude --version` prints `chaude 0.8.0` (C4; main.go:42 prints "chaude " + version).
- type: H.
- depends: U8 PASS, U0a commit done.

### U-rel-2 (S) release review and commit (last)
- sonnet-reviewer once on the 0.8.0 diff (U0a..U-rel-1). Fix findings; re-run acceptance.
- commit: `0.8.0: per-model live child processes, idle rotate at 3599s, cache TTL observe` + final line `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>` (D4, user decision).
- tag: `git tag v0.8.0` ONLY on user request. No push.
- acceptance: `go test ./... -count=1` all ok (no `(cached)`), `go vet ./...`, `go build ./...`.
- depends: U-rel-1.

## 5. ordering (single source; §4 and §7 match this)
U-pre -> [G1 approval] -> U0a -> U3b -> U7a -> U5a -> U5b -> U5d -> U6b -> U7b -> U8 -> U-rel-1 -> U-rel-2.
- U6b depends on U5b, U5d, U6a. U7b depends on U5a, U3b, U7a. U8 depends on U7b, U6b.
- parallel allowed: U7a with U5a (different files). U-pre with U0a staging prep is NOT allowed (B4: baseline must be on the same tree as U0a).
- release (U-rel-1, U-rel-2) runs strictly last, after U8.

## 6. acceptance (all units)
- every unit: `go build ./...` exit 0; `go vet ./...` exit 0; `go test ./internal/harness/... -count=1` PASS (no cache). U5a/U5b/U8 also `go test -race ./internal/harness/ -count=1`.
- TUI units (U5b, U8): `go test -count=1 -race ./internal/harness -run 'ScrollBottom|WheelScroll'` (E5).
- release: `go test ./... -count=1` all ok; `go run ./cmd/chaude --version` = `chaude 0.8.0`; `git tag` v0.8.0 only after user request.
- manual (after U8): `chaude --idle-rotate 5s`; `ps` orphan 0 after exit.
- live tests: $3 cap per task; smoke first; skip if cap mechanism unverified.

## [haiku] steps
Ordered unit IDs (serial). Haiku runs H units only; S units go to sonnet-analyst.
1. U-pre [H]: baseline test + name-to-PASS record + diff hash.
2. U0a [H, staging; only after G1 approval].
3. U3b [H part]: Repl fields and newChildSet() wiring. Tests: TestCacheTTLFlagWiring (if missing), TestRegistryNilWhenNoFile.
4. U7a [H]: expiredZero in clients.go + TestExpiredZeroDeadline.
5. U5a [S -> sonnet-analyst].
6. U5b [S -> sonnet-analyst].
7. U5d [S -> sonnet-analyst].
8. U6b [S -> sonnet-analyst; reviewer sonnet; escalate opus-architect after 2 fails].
9. U7b [S -> sonnet-analyst].
10. U8 [S -> sonnet-analyst].
11. U-rel-1 [H]: only after U8 PASS: version 0.8.0 + CHANGELOG `## 0.8.0`.
12. U-rel-2 [S -> sonnet-reviewer + commit].

## 8. doubts / out of this plan
- 0.8.x follow-up (not in 0.8.0): U8b (T6 reconcile orphan check, /clear CloseAll T7), U5c (DesignPath), U9-budget ($3 ledger, budget.go, --max-budget-usd second layer), U10a (auto TTL default, needs approval; plan keeps 1h), U10b (DECISIONS text).
- A3 resolved in plan: liveChild.Sess stays `closer`; Dead() by type assertion on *Session.
- D2 resolved: trailer is Co-Authored-By Claude Haiku 5.5 (user decision D4). The earlier "matches the given instruction" claim is removed.
- idle-rotate in non-TTY REPL is a no-op; stated in --help (D4). If U8 slips, drop the 3599 line from CHANGELOG and the flag default must not advertise it (D3).
- live gate (TestClassifierLive) cost approval pending (G4).
- A1-A12 assumptions in impl doc §6 remain unverified; A3 (live child rebinding) is riskiest, covered by TestRouteKeepsPreviousChildLive plus a manual 3-model round trip.
- TTL 5m observed-rate: unverified until a live run.
- untracked notes/* files are not part of the release commit unless the user says so.
