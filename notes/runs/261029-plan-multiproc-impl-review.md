# review: 261029-plan-multiproc-impl.md (opus-architect)

date: 261029
verdict: implementable after the fixes below. Decisions 1-6 settle open questions 1-5 except user-only items marked [user].
checked: working tree `go vet ./...` clean, `go test ./...` PASS (harness cached ok). Code read: classifier.go:145-233, core.go:108-135/321-335, repl.go:176-262/450-465/895-935, tui.go:968-980/1114-1124, agentstore.go:94-110, ratestatus.go.

## Decisions

1. Serial dispatch for 0.7.3: YES, ship it, but it does NOT fully meet the user's request.
   - user (v3 §1 [사용자확정]): "여러 개의 별도 Claude Code 클라이언트(세션)가 동시에 실행되는 것". Serial dispatch meets premises 1-2 (separate process per model, own cache, no in-process model switch) but not "동시에 실행" (two models working at once).
   - why ship anyway: childSet/registry/pickChild are strict prerequisites of concurrency and are reused unchanged; no throwaway work.
   - minimum that meets the request (0.8, not 0.7.3): "one background turn per child": when a prompt routes to model B while the active child A is still in a turn, A is parked-live mid-turn (its event pump keeps running via the existing StartIdle/drain pattern), the prompt goes to B, and A's final result is shown as a labeled notice when it arrives. One pending prompt per child max (no FIFO), TUI accepts plain prompts while busy only when they route to a different, idle child. Everything else in plan §1 "제외" stays deferred.
   - wording: CHANGELOG/notices say "모델별 세션 유지(한 번에 하나씩 작업)"; never "동시 실행". Open question 1 becomes a notice to the user, not a blocker: implementation proceeds.

2. DesignPath: written by the Opus child, recorded by chaude; missing = skip silently.
   - creator: the Opus (design) child writes the plan file itself (opus-architect already writes `notes/runs/<YYMMDD>-plan-<topic>.md`). chaude never asks for or generates it.
   - recorder: chaude, in runTurn's existing Write/Edit tool_use branch (repl.go:452-463, same place as `r.pool.noteEdit`): if active model == opus and `file_path` matches `*/notes/runs/*-plan-*.md` and not `*-review*.md`, set the opus row's DesignPath = that absolute path (last write wins). Also in idleEvent tool_use (background turns).
   - missing: (a) no DesignPath recorded -> expiry/resume uses only the normal handoff/snapshot, no attachment, no warning; (b) DesignPath set but file absent or size 0 at attach time -> skip attachment, metrics `{"stage":"design_path","missing":true,"path":...}`, no user-facing error, never block dispatch. No "design first" gating in 0.7.3 (v3 §4 rule stays deferred).
   - attach text (one line appended to pendingSnapshot): `설계 파일: <abs path> (먼저 읽고 이어서 진행)`.

3. Forced kill: rely on stdin EOF, reconcile the registry at next start; no kill-by-PID in 0.7.3.
   - normal exit (TUI quit tui.go:1117-1122, loop exit repl.go:~930): `children.CloseAll()` closes all live children IN PARALLEL (goroutine per Close + WaitGroup; Close already does stdin close -> 15s wait -> Kill). Serial would hang up to 45s. Registry rows -> State=parked, EndedReason="exit", SessionID kept (resumable within deadline).
   - SIGINT/SIGTERM: unchanged (forwarded as interrupt, repl.go:912-920).
   - SIGHUP (terminal closed) / SIGKILL / crash: no cleanup code runs. Children are in chaude's process group (no Setpgid; do NOT add Setpgid) so SIGHUP reaches them directly; on SIGKILL the stdin pipe hits EOF and `claude -p --input-format stream-json` exits [가정, verify below]. claude cleans its own tool subprocesses.
   - registry after forced kill: rows stay State=live (stale) until next start. Add fields `OwnerPID int` (chaude pid) and `ChildPID int`. At startup, next to `r.agents.interruptRunning(false)` (repl.go:903), call `r.reg.reconcile(now, deadline)`: for rows with State=live and `!pidAlive(OwnerPID)` (reuse agentstore.go:95) -> State=parked if SessionID!="" && now-LastAPI<deadline else expired; EndedReason="harness-killed". Rows owned by a live other chaude are untouched.
   - pickChild resume must skip rows whose OwnerPID is alive and != os.Getpid() (another chaude holds that session).
   - acceptance (manual, new): start chaude, route to 2 models, `kill -9 <chaude pid>`, after 20s `pgrep -f 'claude -p --input-format stream-json'` = 0; restart chaude -> clients.json rows State=parked, EndedReason=harness-killed. If orphans remain: add kill fallback (only if ChildPID alive AND `ps -o ppid=` == 1 AND comm contains "claude") as a follow-up, not before.

4. Uncommitted changes: separate commit FIRST, before any 0.7.3 unit starts.
   - content: size-tag classifier (classifier.go, conductor_test.go) + rate usage status line (repl.go setRate/rateStr, tui.go statusLine, ratestatus.go, ratestatus_test.go). Self-contained, sonnet-reviewer PASS (LEDGER 10-29 13:18), vet clean, tests PASS.
   - why separate: U3b/U5/U6b/U8 edit the same files (repl.go, classifier.go, tui.go); mixing makes the 0.7.3 diff unreviewable and un-revertable per feature.
   - order: (a) commit code: `classifier: size tag defaults to large; status line: 5h rate usage` (no version bump), (b) commit `notes/LEDGER.md` + `notes/metrics/2026-10-29.jsonl` as `metrics: 2026-10-29` (matches b6e057f pattern), (c) notes/runs/*.md + notes/research/* in a docs commit or left untracked, (d) then 0.7.3 work. All commits need the user's go-ahead [user]; until then 0.7.3 units must not start on these files (pure units U1a-U1c, U2a, U2b, U3a, U4a, U4b touch only clients.go/children.go/core.go and may start).
   - CHANGELOG `## 0.7.3` lists these two items too (they ship in the 0.7.3 binary).
   - non-blocking before (a): ratestatus.go `if v < 1 { v *= 100 }` shows a fraction 1.0 (100%) as "1%" and a percent value 0.5 as "50%". Check the real scale in `notes/metrics/*.jsonl` rate_limit_info; if fraction-only, use `v <= 1`.

5. DECISIONS:23 "pin" -> auto: needs the user's approval BEFORE the default and the DECISIONS line change. Implementation proceeds meanwhile.
   - why approval: DECISIONS:23 is a user-quoted decision ("그럼 캐시 갱신은 59:59초에 수행하게 하고" -> pin 1h); auto changes cost behavior in overage (5m vs pinned 1h 2x write). That trade-off is the user's.
   - meanwhile: implement `--cache-ttl auto|1h|5m` fully, but default = "1h" (honors DECISIONS:23 as written). Flipping to auto after approval = one-line default change + U10 DECISIONS edit, done together.
   - DECISIONS:23 already contains "auto-fallback to 290s" when usage shows 5m: implement that regardless of approval (idleDeadline uses observed TTL first, see fix U1c).
   - CHANGELOG wording if not approved by release: `캐시 TTL 관측(5m 감지 시 290초로 단축)` instead of `캐시 TTL auto 감지`.
   - question for the user [user]: "캐시 TTL을 1h 고정(현재 결정) 대신 auto(초과 사용 시 5분, 쓰기 2배 회피)로 바꿔도 될까요?"

6. switchSession (classifier.go:204-224): YES, keep the old close+park-id+resume path for the same model; live path only when the model differs.
   - same model = haiku -> haiku worker switch (`rr.Worker != ""`, classifier.go:176-179). Key (cwd, model) allows one live haiku, so a second haiku session cannot be represented in childSet; keep `switchSession("haiku", rr.Worker)` exactly as now.
   - different model (incl. haiku -> non-haiku and back, classifier.go:181-198 and the `r.a.Model != "haiku"` arm of :176): `dispatch(model)` -> Park + pickChild. Also drop the `r.a.Model == "haiku"` restriction at :183 under the live path (any model change uses pickChild).
   - registry/childSet nil -> whole old route() unchanged (TestRouteFallsBackWithoutRegistry).
   - on useLive: `delete(r.parked, model)` so a stale id cannot be resumed later; on Park do NOT write r.parked (registry holds the id).

## Unit-level fixes

7. liveChild must carry per-child state, not only Sess: `liveChild{Sess *Session; Tracker *agentTracker; Model, Role string; LastAPI time.Time; LastCtx int; Files []string; TurnFrom int}`. Park saves r.tracker, r.lastCtx, r.files, start index into r.tr.turns; useLive restores them. Without LastCtx restore the status line ctx and the rotate threshold (repl.go:764) use the other model's ctx. Do NOT call StopIdle on Park (the idle pump started in start() repl.go:212 must keep draining stdout, else the parked child blocks on a full pipe).
8. idleEvent from a non-active session (s != r.sess) must not touch active state: usage -> that liveChild.LastCtx (not r.idle.usage/r.lastCtx); text -> not streamed via r.idleText; buffer and show once on its result as a dim block `[<model> 대기 세션] <text>`; permission prompt label prefixed `[<model>]`. Add test TestIdleEventFromParkedChild (fake events, r.lastCtx unchanged, no idleText call).
9. Idle expiry: only the ACTIVE child gets the in-TTL handoff (`rotate("idle 3599s")`). Non-active live children at deadline: Close (no handoff), State=expired, next use = startNew (+DesignPath line for opus, decision 2). Reason: rotate() reads r.tr.turns/r.sess (mixed models); promoting a parked child just to rotate flips the UI model under the user. Replace plan §2 "해당 자식을 활성으로 올려" with this. Q3 still holds (opus resumes from the design file).
10. Flag name: use the decided `--idle-rotate` (DECISIONS:21/23, default 3599s, 0=off), not a new `--idle-end`. No idle-rotate code exists today (grep: no match), so no compat issue. Update U3b, manual acceptance (`chaude --idle-rotate 5s`), CHANGELOG.
11. U1c `idleDeadline(idleRotate time.Duration, ttl string)`: caller passes `r.ttlSeen` if non-empty else the flag value. "1h" -> idleRotate; "5m" -> min(idleRotate, 290s); "auto" with nothing observed -> 290s; idleRotate 0 -> 0. Test cases: (flag 1h, seen 5m) -> 290s; (flag auto, seen 1h) -> 3599s.
12. One timestamp name: registry `LastAPI` (= main `message_start` time) replaces `LastUsed`; isWarm and idleDue both use it. Rename in U1a/U1b and tests.
13. childKey: resolve `r.a.Cwd == ""` via os.Getwd, then filepath.Abs + EvalSymlinks (macOS /var vs /private/var). TestChildKey: "" and "." give the same key as the cwd.
14. Registry save: re-read file, replace only own rows (cwd, model), keep others; write OwnerPID/ChildPID (Session needs `PID() int` = cmd.Process.Pid, 0 for fakes).
15. Order change: U0(a) commits (decision 4) -> U0(b) TTL gate -> pure units -> serial group. U10 split: U10a after approval = default auto + DECISIONS:23 line; U10b any time = DECISIONS line for serial-dispatch scope and "what counts toward the cap" (agent note, no user decision changed).
16. Tests to add: TestReconcileKilledOwner (decision 3), TestCloseAllParallel (3 fake closers each sleeping 100ms finish < 250ms), TestRouteSameModelWorkerUsesOldPath (haiku worker switch: Close called 1, childSet unchanged), TestDesignPathRecorded (opus Write to notes/runs/x-plan-y.md sets row; -review.md ignored), TestDesignPathMissingSkipped.
17. Agent choice: U6b/U7/U8 are tightly coupled to rotate/runTUI; keep sonnet-analyst but give it this review plus plan §2. U1a-U4b, U9 to haiku-coder as planned. Final sonnet-reviewer over the 0.7.3 diff only (after the separate commit, so the diff excludes classifier/ratestatus work).
18. Must-not-change additions: switchSession same-model body; Session.Close semantics; no Setpgid; `r.agents.interruptRunning` behavior.
