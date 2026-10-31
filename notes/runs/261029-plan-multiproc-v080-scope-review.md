# review: 261029-plan-multiproc-v080-scope.md
date: 261029
reviewer: opus-architect
verdict: plan usable after fixes below. Main defect: wrong sourcing — T6/T7 and U9 specs DO exist (rebind docs), plan marks them missing and invents conflicting U9 rules.
scope: improvements only. no source edited.
markers: [spec] = taken from an existing source (cited). [proposal - user confirm] = not defined in any source; smallest defensible design.

## 0. sourcing (plan header l.5-6, §5 a/b/d)
- fix: add to sources: 261029-plan-multiproc-rebind.md (§1.5 l.36, §2 rows 7-8, §3.2 [결정]) and 261029-plan-multiproc-rebind-review.md (l.21-22 T6/T7, l.36-46 §4 budget, l.56-57 tests, l.65 order).
- fix: remove "gaps: ... NOT defined" line; replace §5 a/b/d with pointers to §1/§2 below.
- fix §5a: resolved, not a user question. clear() (repl.go:1026) already closes the active child (r.sess.Close()); T7 adds parked children. DECISIONS:42 "keep current behavior" applies to busy agents -> drain (see §1.2 busy rule), not to keeping parked children.
- fix §5c: keep (name clash note is correct: impl §4 "U9" = old version bump).

## 1. U8b (plan §1 row U8b, §2 U8b, §4 S2)

### 1.1 T6 reconcile orphan check
- [spec] rebind-review l.21: reconcile checks pidAlive(ChildPID) first; alive -> State=expired, EndedReason="orphan-alive" (no resume). SIGHUP added to signal.Notify -> orderly exit. Manual L4: `ps` child count after ~20s.
- gap (code): ChildPID is never written. clients.go:32 field exists; Upserts at switchchild.go:83, :167, :183, :192 omit it; Upsert (clients.go:104) replaces the whole row, so every Upsert must carry it. Session has no PID accessor (core.go:123 `cmd *exec.Cmd`).
- gap (code): reconcile (clients.go:177) skips non-live rows. Parked rows of a killed harness also have live child processes (parked children are real processes). Without checking them, isWarm (accepts parked) lets a new harness --resume a session whose orphan still runs.
- step 1 core.go: add `func (s *Session) PID() int` -> s.cmd.Process.Pid, 0 when s==nil, cmd==nil or Process==nil.
- step 2 switchchild.go (next to sessID, l.39): add `func childPID(c closer) int` (type-assert *Session like sessID; fakes -> 0). Set `ChildPID: childPID(...)` in the 4 Upserts above (parked row -> parked child's pid; live row l.192 -> r.sess).
- step 3 clients.go reconcile: loop condition becomes `(c.State == clientLive || c.State == clientParked) && !pidAlive(c.OwnerPID)`. Then:
  - `c.ChildPID != 0 && pidAlive(c.ChildPID)` -> State=expired, EndedReason="orphan-alive".
  - else if State==live -> existing rule (parked if SessionID!="" && now-LastAPI<deadline else expired; reason "harness-killed").
  - else (parked, child dead) -> keep parked, EndedReason="harness-killed".
  - ChildPID=0 in every changed row; changed=true.
- [proposal - user confirm] orphan handling = mark only, no kill. Reason: PID reuse could make the harness kill an unrelated process; a false "orphan-alive" only costs a cold start (fail-safe). Alternative (rejected by default): SIGTERM after checking `ps -o command=` contains "claude -p".
- step 4 repl.go start: call `r.reg.reconcile(time.Now(), r.warmDeadline())` once, next to `r.agents.interruptRunning(false)` (repl.go:999; plan says :903 — stale). Must run after wireClients (repl.go:180-188, r.reg set) and before the first route/Upsert. Not per tick. r.reg nil-safe (clients.go:168).
- step 5 repl.go:1010: `signal.Notify(sig, os.Interrupt, syscall.SIGTERM, syscall.SIGHUP)`; SIGHUP takes the same orderly-exit path as SIGTERM (markInterrupted, closeDrains, CloseAll, MarkEnded "harness-exit"). Verify the existing SIGTERM branch does all four; if not, the SIGHUP branch calls them explicitly.

### 1.2 T7 /clear, import, /model
- [spec] rebind-review l.22: on /clear and applyImport, CloseAll parked children and mark registry rows expired "clear". /model <name>: keep rotate, but first Take+Close/drain any parked child of the target key.
- fix plan §1 file: /clear logic is repl.go clear() (repl.go:1026), not tui.go. tui.go:951-952 and :1147-1148 only call r.clear(). resume.go:92 calls r.clear() before applyImport, so one hook in clear() covers import (no separate import edit).
- step 1 repl.go clear(), before `r.sess.Close()`:
  - `for _, k := range r.children.Expired(time.Now(), 0) { if c, ok := r.children.Take(k); ok { r.retire(c); r.reg.MarkEnded(r.a.Cwd, c.Model, "clear", time.Now()) } }` (nil-check r.children; Expired(now,0) = all keys, children.go:103).
  - use retire (switchchild.go:88), not closeAllChildren: retire drains a child whose Tracker has running agents (DECISIONS:42 "keep current behavior") and closes the rest.
  - `r.parked = map[...]{}` (classifier.go:254-281 / switchchild.go:141): those SIDs are resumed via --resume and would revive pre-clear context.
  - after `r.sess.Close()`: `r.reg.MarkEnded(r.a.Cwd, r.a.Model, "clear", time.Now())` for the active row (start() re-Upserts live).
- [proposal - user confirm] busy parked child on /clear = drain via retire (agents finish, reply shown as notice), not hard Close. Smallest change consistent with DECISIONS:42 and sweepIdle's busy rule (idlesweep.go).
- step 2 tui.go:1159 /model: before `r.rotate(...)`: `if c, ok := r.children.Take(childKey(r.a.Cwd, f[1])); ok { r.retire(c); r.reg.MarkEnded(r.a.Cwd, f[1], "model-switch", time.Now()) }`. Keeps rotate semantics (handoff snapshot); prevents a stale parked child of the target later being bound over the fresh one.

### 1.3 U8b tests (replace plan §2 U8b test list)
- [spec] rebind-review l.56: TestReconcileOrphanAlive, TestClearClosesParked. Others below = [proposal] additions.
- TestReconcileKilledOwner (clients_test.go:111): must still PASS unchanged (ChildPID 0 -> old rule).
- TestReconcileOrphanAlive: live row OwnerPID=0 (dead), ChildPID=os.Getpid() (alive) -> expired "orphan-alive", ChildPID=0. Plus parked row same pids -> expired "orphan-alive". Plus parked row ChildPID=0 -> stays parked, reason "harness-killed".
- TestReconcileStartupWired: fake registry file with one dead-owner live row; run start path (newSessionFn seam, repl.go:200) -> row no longer live.
- TestClearClosesParked: childSet with 2 fake closers (other models), one fake active; r.clear() -> each fake Close called once, rows expired "clear", r.parked empty, active closed once.
- TestClearDrainsBusyParked: parked child with Tracker.running()>0 and MaxDrain>0 -> lands in r.draining, not closed.
- TestModelSwitchDropsParkedTarget: parked child key (cwd,"opus") + /model opus -> Take'd and retired before rotate; row expired "model-switch".
- TestChildPIDRecorded: after park/bind with a real-ish Session stub (PID()!=0 path) rows carry ChildPID; fakes -> 0.
- SIGHUP: manual only (L4); no unit test.
- fix plan §2 run regex: `-run 'Reconcile|ClearCloses|ClearDrains|ModelSwitchDrops|ChildPID'` (plan's 'ClearCloseAll' matches no spec'd test).
- fix plan §2 manual: two branches, not one. kill -9 chaude:
  - child exited by itself (stdin EOF) -> restart: row parked (if recent) EndedReason=harness-killed.
  - child still alive -> restart: row expired EndedReason=orphan-alive; `pgrep -f 'claude -p --input-format stream-json'` count recorded either way.
- keep: race test `-run 'ScrollBottom|WheelScroll'` (tui.go touched by /model step).

## 2. U9 budget ledger (plan §1 row U9, §2 U9, §4 S3, §3 step 5/7)

### 2.1 conflicts to fix
- flag name/default: plan says `--max-budget-usd` default 3. [spec] rebind §3.2 S3: harness flag `--task-budget-usd`, default off (0); when >0 it appends `--max-budget-usd <cap>` to opts.Extra (repl.go:213) as backstop. Rename everywhere.
- enforcement: plan says "flag if --help lists it, else counter". [spec] rebind-review l.36-46: ledger is primary (pre-gate + mid-turn interrupt); CLI flag is only a backstop. --max-budget-usd confirmed (research-budget-flag.md, CLI 2.1.312), so the "else" branch is moot. Post-call-only stop rejected (one cold opus 270k turn with 1h cache write ≈ $3).
- hard stop: plan says "Close child". [spec] rebind §3.2 S1: refuse the next call, close no child, message `[budget] $X / $3 reached, task stopped` (show actual cap), metrics stage=budget.
- flag location: plan says cmd/chaude/main.go. Flags live in repl.go:966 area (fs.DurationVar idle-rotate); Args field next to IdleRotate (repl.go:63): `TaskBudgetUSD float64`.
- CHANGELOG §3 step 5: "비용 상한 $3 (--max-budget-usd)" -> "작업당 비용 상한 옵션 (--task-budget-usd, 기본 꺼짐)". Commit subject §3 step 7: "$3 task budget cap" -> "optional per-task budget cap".

### 2.2 what is counted
- [spec] unit = USD from result.total_cost_usd, which is cumulative per process -> charge the delta vs the last value seen for that *Session. One-shot calls charge their full cost.
- charge points (all existing cost reads):
  - runTurn just before `return` (repl.go:546) when result!=nil: covers user turns (handle, repl.go:802 reads same result) AND handoff turns (makeSnapshotN repl.go:627 calls runTurn; repl.go:802 alone would miss them). Do not also charge at :802 (double count).
  - idleFinish background result (repl.go:366, has s *Session).
  - parked child background result (livechild.go:66 `t == "result"`): charge(c.Sess.(*Session), ev total_cost_usd).
  - classifier one-shot (classifier.go:201): chargeOneShot(cost).
  - drained sessions: not charged (agents finishing after switch; [proposal - user confirm] accept as known gap, backstop flag covers it per process).
  - /usage (tui.go:699): no-session-persistence status call; [proposal] chargeOneShot if total_cost_usd present, else ignore.
- [spec] subscription login: total_cost_usd is a nominal proxy (rebind §3.2); counted the same.
- [proposal - user confirm] resume baseline: research-budget-flag.md says a resumed session's result includes earlier spend (v2.1.277+). New *Session started with --resume: seed last[s] from in-process `lastBySID[sessionID]` when known; unknown -> baseline 0 (overcharge = stops early, fail-safe). 0.01 USD smoke C3 confirms the real behavior; adjust only after C3.

### 2.3 where it is stored
- [spec] new file internal/harness/budget.go, in memory on Repl (`r.budget *budget`), no file, no registry field.
- type (rebind-review l.57, extended by SID map for resume):
  ```
  type budget struct {
      cap, spent, running float64      // cap<=0 = off
      last      map[*Session]float64    // last total_cost_usd per process
      lastBySID map[string]float64      // seed for --resume processes
  }
  func (b *budget) reset()                         // spent=0, running=0; keep last/lastBySID
  func (b *budget) charge(s *Session, total float64) // d=total-last[s]; d<0 -> d=total; spent+=d; last[s]=total; lastBySID[s.SessionID]=total
  func (b *budget) chargeOneShot(cost float64)
  func (b *budget) allow(est float64) bool         // cap<=0 -> true; false if spent>=cap || spent+est>cap
  func (b *budget) over(running float64) bool      // cap>0 && spent+running>=cap
  ```
  nil *budget methods = no-op / allow true (cap off path, tests).
- [spec] task boundary: reset() at the top of handle() (repl.go:769) for each user prompt — matches impl §4 l.106-108 user instruction (작업당 $3, 합산 아님). Background/parked charges between prompts go to the current task.
- concurrency: charge is called from runTurn (main), idleFinish and parkedIdleEvent goroutines -> guard with sync.Mutex inside budget.

### 2.4 what it gates
- S1 pre-gate [spec]: before classify (handle, before r.route at repl.go:773, est=0) and at the top of runTurn (repl.go:432, before Send; covers handoff turns). Refused -> runTurn returns ("", nil, nil); handle prints `[budget] $X / $CAP reached, task stopped`, writes metrics stage=budget, skips rotate/dispatch for this prompt. No child closed.
- est(next) [spec]: lastCtx × (cache-read price if warm else cache-write price: 1h 2x, 5m 1.25x of input) + 4k × output price, per-model table in budget.go.
- [proposal - user confirm] price table values: no source in notes has per-model prices. haiku-researcher fills them from the official Anthropic pricing page into notes/FACTS.md with URL + date before S3; budget.go copies them. Missing model -> est=0 (S1 then checks spent only; S2/S3 still protect).
- S2 mid-turn [spec]: in runTurn on each main assistant usage event (lastUsage update), running = est from that usage; if b.over(running) -> r.sess.Interrupt() (core.go:222), message as S1.
- S3 backstop [spec]: cap>0 -> start() appends `--max-budget-usd <cap>` to opts.Extra (repl.go:213; note it currently assigns `opts.Extra = []string{...}` -> must append, not overwrite). Per-process cap, so it cannot enforce a per-task total alone.
- [proposal - user confirm] default off for users: on subscription total_cost_usd is nominal and a third-party report says the CLI cap stops runs despite zero API cost (research-budget-flag.md). Live tests by agents pass `--task-budget-usd 3` explicitly (impl §4 l.106-108).

### 2.5 U9 tests (replace plan §2 U9)
- [spec] rebind-review l.57: TestBudgetDeltaPerProcess, TestBudgetGateBeforeSend, TestBudgetInterruptMidTurn.
- [proposal] add: TestBudgetResetPerPrompt, TestBudgetOffAllows (cap 0 / nil), TestBudgetResumeBaseline (unknown SID -> baseline 0; known SID -> seeded), TestBudgetExtraAppends (cap>0 -> opts.Extra has both --append-system-prompt and --max-budget-usd).
- run: `go test -count=1 ./internal/harness/ -run 'Budget' -v`.

## 3. ordering / gates (plan §3, §4)
- fix: G4 + rebind-review l.65: U9 PASS and the 0.01 USD smoke (haiku-runner, <$0.05; records result subtype, process exit, --resume counter reset = C3) must PASS before any L-item or live run. Add as explicit step between §3 step 4 and step 5; release (R1-R4) needs no live run.
- keep: S1 -> S2 -> S3 serial (shared repl.go).
- add to S2 brief: read rebind-review l.21-22 and this file §1; S3 brief: read rebind §3.2 and this file §2 (plan currently points S3 only to research-budget-flag.md).
- stale refs: repl.go:903 -> :999 (interruptRunning); repl.go:810 loop defer -> ~:873; recheck before editing.

## 4. open questions for the user (one block)
- U9 기본값: --task-budget-usd 기본 꺼짐, 라이브 테스트에서만 3 지정 — 확인 필요.
- 고아 프로세스: 표시만 하고 종료(kill)하지 않음 — 확인 필요.
- /clear 때 작업 중인 parked 자식: 강제 종료 대신 drain — 확인 필요.
- --resume 비용 기준선: 모르면 0 (일찍 멈추는 쪽) — 확인 필요.
- 단가표: 공식 가격 페이지에서 haiku-researcher가 채움 — 확인 필요.

추천: §0-§2 수정 반영 후 S2(U8b)부터 진행하고, U9는 §4 다섯 항목 확인을 받은 뒤 시작하시는 것을 권합니다.
