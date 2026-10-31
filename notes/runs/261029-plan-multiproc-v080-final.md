# plan (final): v0.8.0 scope add (U8b, U5c, U9) + release
date: 261029
status: plan only. no source edited, no commit, no amend, no tag move, no push. execution follows section order.
base: HEAD 5e415b5 "0.8.0: per-model live child processes, idle rotate at 3599s, cache TTL observe" (local, unpushed). tag v0.8.0 = annotated tag object 442e768 -> peels to 5e415b5 (local only, unpushed).
sources (merged): 261029-plan-multiproc-v080-scope.md (base), 261029-plan-multiproc-v080-scope-review.md (review, opus-architect), 261029-plan-multiproc-finish-review.md, 261029-plan-multiproc-rebind.md (§1.5 l.36, §2 rows 7-8, §3.2), 261029-plan-multiproc-rebind-review.md (l.21-22 T6/T7, l.36-46 budget, l.56-57 tests, l.65 order), 261029-research-budget-flag.md, notes/DECISIONS.md (:42).
markers: [spec] = taken from a cited source. [user] = user decision, final. [proposal] = smallest defensible design, not in any source. [S] = sonnet-analyst unit. [H] = haiku-executable step.

## 0. user decisions (final, do not change)
1. U9 budget feature is disabled by default (flag --task-budget-usd, off unless set). [user]
2. Orphan processes are only displayed; never killed or cleaned up. [user]
3. Before /clear runs, the code must hand off to the next session; verify such handoff code exists and add it if missing. [user]
4. Resume cost is always 0 (the U8b/U9 resume-cost path must return 0). [user]
5. Price table source: docs.claude.com token/$ pricing. [user]

Required fixes (from review, mandatory in U8b):
- BUG-1: ChildPID must be recorded. Currently never written: clients.go:32 field exists; Upserts at switchchild.go:83, :167, :183, :192 omit it; Upsert (clients.go:104) replaces the whole row, so every Upsert must carry it. Session has no PID accessor (core.go:123 `cmd *exec.Cmd`). Fix: add `func (s *Session) PID() int` (core.go; 0 when s nil, cmd nil or Process nil), add `childPID(c closer) int` (switchchild.go next to sessID l.39; fakes -> 0), set `ChildPID: childPID(...)` in all four Upserts.
- BUG-2: reconcile must not resume parked rows whose child is still alive. clients.go:177 skips non-live rows, so a parked row of a killed harness whose child still runs would pass isWarm and `--resume` a session with a live orphan. Fix: reconcile loop covers live AND parked rows; `ChildPID != 0 && pidAlive(ChildPID)` -> State=expired, EndedReason="orphan-alive", never parked, never resumed.

## 1. unit table
legend: H = Haiku-executable. S = sonnet-analyst (touches repl.go / tui.go / switchchild.go / core.go / classifier.go; gate rule finish §1 G1 + sonnet-analyst split).

| ID | type | files | functions / names | depends |
|---|---|---|---|---|
| U5c | S | internal/harness/clients.go (DesignPath field, exists per impl §2), internal/harness/repl.go (runTurn Write/Edit branch ~repl.go:452-463, next to r.pool.noteEdit), idleEvent tool_use path | recordDesignPath(model, filePath string) (new, name proposed); tests TestDesignPathRecorded, TestDesignPathMissingSkipped | U5a (done), U-rel PASS |
| U8b | S | internal/harness/core.go (PID()), internal/harness/switchchild.go (childPID, 4 Upserts, retire usage), internal/harness/clients.go (reconcile loop), internal/harness/repl.go (start() reconcile call ~:999; signal.Notify ~:1010; clear() ~:1026; /model handler path), internal/harness/tui.go (/model handler ~:1159 only; /clear handler just calls r.clear()) | PID(), childPID(), reconcile(now, deadline) extended, clear() hook, /model drop-parked-target | U8 (done), U1a reconcile (exists) |
| U9 | S | internal/harness/budget.go (new), internal/harness/repl.go (Args field next to IdleRotate repl.go:63; flag in fs area ~:966; start() opts.Extra append ~:213; handle ~:769/:773; runTurn ~:432/:546; idleFinish ~:366; parked result livechild.go:66), internal/harness/classifier.go (:201 one-shot), internal/harness/core.go (Interrupt core.go:222) | budget type, charge, chargeOneShot, allow, over, reset; flag --task-budget-usd (default 0 = off) | none (independent of U5c/U8b except repl.go serial order); G4 + smoke gate for live test |

H items: none of the three units is H (all touch repl.go / tui.go / need design). Haiku runs only §[haiku] steps.

## 2. per-unit acceptance
common: `cd /Users/leekwangseok/PycharmProjects/claude-harness && go build ./... && go vet ./internal/harness/ && go test ./internal/harness/... -count=1` exit 0 (no "(cached)").
full gate before any commit: `go build ./... && go vet ./... && go test ./... -count=1` exit 0.

### U5c [S]
- `go test -count=1 ./internal/harness/ -run 'DesignPath' -v` PASS (TestDesignPathRecorded, TestDesignPathMissingSkipped).
- Rules: opus active model + Write/Edit file_path matches `*/notes/runs/*-plan-*.md` and not `*-review*.md` -> DesignPath = absolute path (last write wins). Non-opus model: no record.
- Missing or 0-byte file at attach time -> skip attach, metrics `{"stage":"design_path","missing":true,"path":...}` only, no user error, no dispatch block.
- Attach line: `설계 파일: <abs path> (먼저 읽고 이어서 진행)` appended to pendingSnapshot end.
- manual: none required.

### U8b [S]
- `go test -count=1 ./internal/harness/ -run 'Reconcile|ClearCloses|ClearDrains|ModelSwitchDrops|ChildPID' -v` PASS. (review fix: plan's 'ClearCloseAll' matched no spec'd test.)
- TestReconcileKilledOwner (clients_test.go:111) must PASS unchanged (ChildPID 0 -> old rule).
- Reconcile rule (final):
  - loop over State=live || State=parked, where !pidAlive(OwnerPID).
  - ChildPID != 0 && pidAlive(ChildPID) -> State=expired, EndedReason="orphan-alive" (display only, never killed; user decision 2). Never parked, never resumed (BUG-2).
  - else State=live -> parked if SessionID!="" && now-LastAPI<deadline, else expired; EndedReason="harness-killed".
  - else State=parked (child dead) -> keep parked, EndedReason="harness-killed".
  - ChildPID=0 in every changed row; changed=true. Live owner rows untouched.
- Reconcile runs once at start (repl.go, after wireClients repl.go:180-188 sets r.reg, before first route/Upsert; next to r.agents.interruptRunning(false) ~repl.go:999). Not per tick. r.reg nil-safe (clients.go:168).
- Orphan display: orphan-alive rows shown in status/notice only. No kill, no SIGTERM, no `ps`-based cleanup (user decision 2).
- /clear (user decision 3): handoff must exist before /clear.
  - S2 step 0: grep repl.go/tui.go/handoff code (makeSnapshotN repl.go:627, handoff turns) and confirm the handoff path runs before clear(). If missing, add it: clear() must call the handoff (snapshot for the next session) before any close. Verify with a unit test TestClearHandsOffFirst (handoff called before CloseAll/retire).
- clear() hook (repl.go:1026; resume.go:92 and applyImport use the same path):
  - before `r.sess.Close()`: for each parked key in r.children.Expired(now, 0): Take, r.retire(c) (drains busy child per DECISIONS:42, closes idle), r.reg.MarkEnded(cwd, model, "clear", now).
  - `r.parked = map[...]{}` so pre-clear SIDs are not resumed.
  - after `r.sess.Close()`: r.reg.MarkEnded(cwd, model, "clear", now) for the active row.
- /model (tui.go:1159, before r.rotate): if parked child of target key exists -> Take, r.retire, MarkEnded "model-switch". Rotate semantics kept.
- BUG-1 fix verified by TestChildPIDRecorded.
- SIGHUP: signal.Notify(sig, os.Interrupt, syscall.SIGTERM, syscall.SIGHUP) (repl.go ~:1010); SIGHUP runs the same orderly-exit as SIGTERM (markInterrupted, closeDrains, CloseAll, MarkEnded "harness-exit"). Manual only.
- tests (new, [proposal] except where [spec]):
  - [spec] TestReconcileOrphanAlive: live row OwnerPID=0, ChildPID=os.Getpid() -> expired "orphan-alive"; parked row same pids -> expired "orphan-alive"; parked row ChildPID=0 -> parked, "harness-killed".
  - [spec] TestClearClosesParked: 2 fake closers (other models) + 1 fake active; r.clear() -> each Close once, rows expired "clear", r.parked empty, active closed once.
  - [proposal] TestReconcileStartupWired: fake registry file, dead-owner live row; start path (newSessionFn seam repl.go:200) -> row not live.
  - [proposal] TestClearDrainsBusyParked: parked child with Tracker.running()>0 and MaxDrain>0 -> r.draining, not closed.
  - [proposal] TestModelSwitchDropsParkedTarget: parked (cwd,"opus") + /model opus -> retired before rotate; row expired "model-switch".
  - [proposal] TestChildPIDRecorded: park/bind with Session PID()!=0 -> rows carry ChildPID; fakes -> 0.
  - [proposal] TestClearHandsOffFirst (user decision 3).
- race: `go test -race -count=1 ./internal/harness -run 'ScrollBottom|WheelScroll'` PASS (tui.go touched).
- manual (two branches; record pgrep count either way):
  - `kill -9 <chaude pid>`; child exited by itself (stdin EOF) -> restart: row parked (if recent) EndedReason=harness-killed.
  - child still alive -> restart: row expired EndedReason=orphan-alive; process left running and displayed (not killed).
  - `pgrep -f 'claude -p --input-format stream-json'` count recorded in both branches.

### U9 [S]
- `go test -count=1 ./internal/harness/ -run 'Budget' -v` PASS.
- Flag: `--task-budget-usd` (harness flag), default 0 = off (user decision 1). When >0 it is the cap in USD per task. Name `--max-budget-usd` is NOT used for the harness flag (it is only the CLI backstop).
- [spec] Ledger is primary; CLI flag `--max-budget-usd` is a backstop only: when cap>0, start() appends `--max-budget-usd <cap>` to opts.Extra (repl.go:213; currently assigns, must append, not overwrite). Confirmed in research-budget-flag.md (CLI 2.1.312). Per-process cap, cannot enforce per-task total alone.
- [spec] Type (budget.go, in memory on Repl `r.budget`, no file, no registry field):
  ```
  type budget struct {
      mu        sync.Mutex
      cap, spent, running float64      // cap<=0 = off
      last      map[*Session]float64    // last total_cost_usd per process
  }
  func (b *budget) reset()                          // spent=0, running=0; keep last
  func (b *budget) charge(s *Session, total float64) // d=total-last[s]; d<0 -> d=total; spent+=d; last[s]=total
  func (b *budget) chargeOneShot(cost float64)
  func (b *budget) allow(est float64) bool          // cap<=0 -> true; false if spent>=cap || spent+est>cap
  func (b *budget) over(running float64) bool       // cap>0 && spent+running>=cap
  func resumeBaseline() float64                     // [user] always 0
  ```
  nil *budget methods = no-op / allow true.
- [user] Resume cost is always 0: a new Session started with --resume gets baseline resumeBaseline() = 0; no seeding from prior SID. Known risk: research-budget-flag.md says resumed results include earlier spend (v2.1.277+), so resumed tasks may be over-counted (fail-safe: stops early) or under-counted if the CLI resets; verify with the 0.01 USD smoke (C3) and report, do not change the rule.
- [spec] Counted unit = USD from result.total_cost_usd (cumulative per process) -> charge delta vs last[s].
- Charge points (all existing cost reads, no double count):
  - runTurn just before `return` (repl.go:546) when result!=nil: user turns and handoff turns (makeSnapshotN repl.go:627). Do not also charge at repl.go:802.
  - idleFinish background result (repl.go:366).
  - parked child background result (livechild.go:66 `t == "result"`): charge(c.Sess.(*Session), total_cost_usd).
  - classifier one-shot (classifier.go:201): chargeOneShot(cost).
  - /usage (tui.go:699): chargeOneShot if total_cost_usd present, else ignore. [proposal]
  - drained sessions after switch: not charged; known gap [proposal], backstop flag covers per process.
- [spec] Task boundary: reset() at top of handle() (repl.go:769) for each user prompt.
- [spec] Concurrency: charge called from runTurn, idleFinish, parked goroutines -> sync.Mutex in budget.
- Gates:
  - S1 pre-gate [spec]: in handle before r.route (repl.go:773, est=0) and at top of runTurn (repl.go:432, before Send; covers handoff turns). Refused -> runTurn returns ("", nil, nil); handle prints `[budget] $X / $CAP reached, task stopped`; metrics stage=budget; no child closed.
  - est(next) [spec]: lastCtx x (cache-read price if warm, else cache-write price: 1h 2x, 5m 1.25x of input) + 4k x output price. Price table from docs.claude.com (user decision 5). Missing model -> est=0.
  - S2 mid-turn [spec]: in runTurn on each main assistant usage event, running = est; if b.over(running) -> r.sess.Interrupt() (core.go:222), same message as S1.
  - S3 backstop: `--max-budget-usd <cap>` appended as above.
- Price table: [user] source docs.claude.com token/$ pricing page only. haiku-researcher records values with URL and date into notes/FACTS.md before S3; budget.go copies them. No other source.
- Tests:
  - [spec] TestBudgetDeltaPerProcess, TestBudgetGateBeforeSend, TestBudgetInterruptMidTurn.
  - [proposal] TestBudgetResetPerPrompt, TestBudgetOffAllows (cap 0 and nil), TestBudgetResumeBaseline (always 0 regardless of SID, user decision 4), TestBudgetExtraAppends (cap>0 -> opts.Extra has both --append-system-prompt and --max-budget-usd; cap 0 -> no flag).
- run: `go test -count=1 ./internal/harness/ -run 'Budget' -v`.
- live check: 0.01 USD smoke only (haiku-runner, <$0.05; records result subtype, process exit, --resume counter behavior = C3) with user OK (G4). Not part of the acceptance above.

## 3. ordering / gates
- serial: S1 (U5c) -> S2 (U8b) -> S3 (U9). shared repl.go; no parallel S units.
- S2 brief: read rebind-review l.21-22, this file §1 and §2 U8b; verify handoff-before-/clear (user decision 3).
- S3 brief: read rebind §3.2, research-budget-flag.md, this file §2 U9. Price source = docs.claude.com only.
- G4 + smoke: U9 PASS and 0.01 USD smoke PASS before any live run or L-item. Release (R1-R4) needs no live run.
- stale refs (recheck before editing): repl.go:903 -> ~:999 (interruptRunning); repl.go:810 loop defer -> ~:873; tui.go:951-952 and :1147-1148 only call r.clear().

## [haiku] steps
Ordered. Haiku runs H steps only. S units go to sonnet-analyst (§4 order above). Haiku fails a step 2x -> stop and report, do not improvise.
S = sonnet-analyst step (wait for PASS report before next step). H = Haiku step.

1. H baseline: `go build ./... && go vet ./... && go test ./... -count=1` on HEAD 5e415b5 tree; record exit codes in notes/runs/261029-check-v080-scope-baseline.md (terse key: value). Stop if any FAIL.
2. S U5c (S1): implement per §2 U5c. Wait for PASS.
3. S U8b (S2): step 0 verify handoff-before-/clear (add if missing), then BUG-1, BUG-2, reconcile, clear hook, /model hook per §2 U8b. Wait for PASS.
4. S U9 (S3): implement per §2 U9 with default off, resumeBaseline 0, price table from docs.claude.com. Wait for PASS.
5. H smoke gate (G4): only with user OK; 0.01 USD smoke, record in notes/runs/261029-check-v080-smoke.md. No live run without OK.
6. H R1 CHANGELOG: edit CHANGELOG.md top `## 0.8.0` section; add one bullet each for: DesignPath (opus 설계 파일 첨부); 시작 시 고아 프로세스 표시 (종료하지 않음), /clear 시 parked 자식 정리 및 핸드오프 확인; 작업당 비용 상한 옵션 (--task-budget-usd, 기본 꺼짐). Keep Korean, match existing 0.8.0 bullets. Forbidden words: "multi-client", "동시 실행". Do not rewrite existing bullets. Do not touch 0.6.7-0.7.2 gap. Version stays `0.8.0` (cmd/chaude/main.go:16 unchanged). If U8b did not PASS, drop its bullet.
7. H R1 verify: `go run ./cmd/chaude --version` prints `chaude 0.8.0`; `go build ./... && go vet ./... && go test ./... -count=1` exit 0.
8. H R2 amend (user decision): amend the local unpushed commit 5e415b5 (HEAD). Stage by explicit path only: `git add CHANGELOG.md cmd/chaude/main.go internal/harness/ <budget.go>`. Never `git add -A`; never add notes/metrics or untracked notes/*. Subject: `0.8.0: design path attach, orphan display on start, /clear parked cleanup, optional per-task budget cap`. Final line: `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`. Use `git commit --amend -F <msgfile>` with msgfile outside repo (e.g. /tmp/v080-msg.txt).
9. H R3 tag move (local only): `git tag -d v0.8.0` then `git tag -a v0.8.0 -m "chaude 0.8.0" <NEW_SHA>` (NEW_SHA from `git rev-parse HEAD`). Verify `git rev-parse v0.8.0^{commit}` = NEW_SHA.
10. H R4 no push: confirm `git status -sb` shows ahead of origin; never run `git push` or `git push --tags`.
11. H R4 record: notes/runs/261029-haiku-coder-release-v080-final.md (terse key: value: old sha 5e415b5, new sha, tag sha, test counts, changed paths).

## 4. [sonnet] S units (sonnet-analyst; reviewer sonnet-reviewer; 2 fails -> opus-architect)
- S1 U5c: per §2 U5c. Read impl §2 DesignPath + §4 tests, runTurn repl.go:440-470.
- S2 U8b: per §2 U8b. Read rebind-review l.21-22, this file §2 U8b, finish §2 (l.55). Handoff check first (user decision 3).
- S3 U9: per §2 U9. Read rebind §3.2, research-budget-flag.md, this file §2 U9.
- order: S1 -> S2 -> S3 (serial).

## 5. decisions applied (no open questions)
- U9 default off: applied (§0-1, §2 U9).
- orphans display only: applied (§2 U8b).
- /clear handoff verify-or-add: applied (§2 U8b step 0, test TestClearHandsOffFirst).
- resume cost 0: applied (resumeBaseline, §2 U9).
- price table docs.claude.com: applied (§2 U9, step 5 smoke).
- review resolved items: T6/T7 spec sourced from rebind docs; /clear drains busy parked child (retire) per DECISIONS:42; reconcile flag name `--task-budget-usd`.

## 6. doubts
- Amend of 5e415b5 and tag move rewrite local-only history; OK only if v0.8.0 never fetched and nothing pushed (verify `git ls-remote --tags origin` before R2).
- resume cost = 0 may over-count or under-count resumed tasks; smoke C3 must report it.
- Handoff code for /clear: existence unknown until S2 step 0 greps it.
- v0.8.0 CHANGELOG bullet "--idle-rotate (TUI only)" not re-verified against --help.
- full `go test -count=1` run for 5e415b5 not archived; baseline step 1 fixes that.
- price values from docs.claude.com not yet fetched; est=0 for missing models until then.
