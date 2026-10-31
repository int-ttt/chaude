# 261030 plan: live sessions (fixed 4 + 4 coder standby)
status: design only, not implemented. final version (self-review applied; see 261030-plan-live-sessions-review.md R1-R10)
source: user brief 261030; code facts from haiku-scout grep 261030 (line numbers below). items tagged [inferred] are not verified in code.

## goal
- fixed config (user decision, do not change): main=opus, tui=haiku, coder=haiku, prompt=haiku (4 base sessions) + 4 coder standby slots = 8 slots max.
- tui and prompt must be separate sessions. tui output must never reach the opus main context.

## facts used (from grep, 261030)
- children.go:13-16 childKey(cwd, model) -> key has no role. tui/coder/prompt are all haiku in same cwd -> same key -> collide.
- dispatch.go:57-61 pickChild(set, reg, key, cwd, model, limit, now, deadline); order warm -> parked -> new (per reviewer).
- dispatch.go:11-12 DefaultMaxWorkers = 3; dispatch.go:43 on cap: "워커 세션 상한(%d개)에 도달해 새 세션을 시작하지 않습니다" (refuses).
- repl.go:1091 -max-workers flag (env CHAUDE_MAX_WORKERS). reviewer: not used in production [inferred, call sites not read].
- repl.go:175 parked map[string]string model -> session id; classifier.go:258-285 parked[model]. keyed by model only.
- livechild.go:171 injects "## While this session was parked" digest on resume.
- repl.go:1168-1169 /clear path calls endAllParked("clear").
- orchws.go:32-43 wsMaxTurns=20, wsMaxCtx=40000, wsMaxIdle=270s -> worker expired.
- repl.go:1089 -idle-rotate default 3599s (DECISIONS:23).
- DECISIONS:16 "[rotate] 261028 user: 최대 상한은 없애자... single rule, no hard cap; ctx >= 270000" -> tag is [rotate] (context size), now 150k by DECISIONS:46.
- clients.go:24 Role "design|exec".

## decisions
### D1 key design
- problem: role missing from key; 5 coders also need distinct keys.
- options: (1) key=cwd|model|role, (2) key=cwd|model|role|slot, (3) separate cwd per role (worktree dirs).
- choice: (2). (1) collides 5 coders; (3) changes file paths for tui/prompt and splits caches.
- format: cwd + "\x00" + model + "\x00" + role + "\x00" + strconv.Itoa(slot). slot=0 for main/tui/prompt; coder slot 0 (base) .. 4 (standby).
- pickChild searches only same role. never reuse a session across roles (tui<->prompt isolation).

### D2 role caps (replaces "no real cap")
- table: main=1 (+1 parked during model switch, existing behavior), tui=1, prompt=1, coder=5 (1 base + 4 standby). total live <= 8 (+ parked main).
- enforcement in pickChild via roleCap(role). new action actQueue when coder cap reached (replaces refuse message at dispatch.go:43 for coder role).
- -max-workers: keep flag name and env CHAUDE_MAX_WORKERS; new meaning = coder pool cap; default 5; valid 1..5; outside range -> error "-max-workers는 1~5 사이여야 합니다 (현재 %d)". DefaultMaxWorkers renamed DefaultCoderCap = 5.
- standby slots are lazy: spawned on demand, not pre-warmed. idle > wsMaxIdle (270s) -> end process, slot freed. base coder slot 0 also follows wsMaxIdle but is respawned lazily on next task.

### D3 coder work assignment: bundle vs one-per-task
- options: (1) one task per coder session, (2) bundle everything into one coder, (3) bundle by shared file set / same mechanical change, parallel across disjoint sets.
- trade-off: (1) max parallelism but each new session pays startup + system/tool prefix + rereads same files [inferred cost]; same-file parallel edits conflict. (2) cheapest startup but serial, one failure blocks all, hits wsMaxCtx 40000 fast. (3) shares read context where it matters, parallel where files are disjoint.
- choice: (3).
- rules (in order):
  1. task.Files overlaps a file leased by an active coder -> append to that coder's queue (same coder, serial). one writer per file.
  2. task.DependsOn names a queued task -> same coder as that task.
  3. same task.Kind (e.g. same rename/format/pattern) as an active coder with queue < 5 and ctx < 24000 (60% of wsMaxCtx) -> append.
  4. free coder slot (< cap) -> new coder (lazy spawn), lease task.Files.
  5. else -> global FIFO queue; dispatch when any coder finishes a task.
- bundle limit per coder: 5 queued tasks or ctx >= 24000, whichever first; then stop appending (session continues until orchws expiry).
- lease released when task reports done/fail.

### D4 prompter mode
- options: (A) one-shot `claude -p` per request, no live prompt session; (B) live prompt session, /exit + relaunch `claude` after every prompt output; (C) live prompt session, each request runs haiku-prompter as one-shot subagent (Agent tool) inside it; prompt session replaced (/exit + relaunch) only at orchws expiry (turns>=20 or ctx>=40000 or idle>270s).
- (A) leaves the fixed prompt slot unused. (B) gives clean context but pays process start + cache write each prompt; prompt prefix may still hit cache across processes within TTL [inferred, unverified], so the gain over C is small while latency is added every time. (C) subagent context is already fresh per request; only the final per-round log enters the prompt session.
- choice: (C). fallback to (B) only if measurement shows leftover context from earlier requests changes later answers.
- /clear is never used or suggested. replacement = send /exit to the child, wait exit (10s, then kill), spawn new `claude` with same role/slot key. no handoff for prompt (stateless between requests).
- prompt output goes to TUI only, never to main.

### D5 conflict with earlier "상한 제거" decision
- DECISIONS:16 is tagged [rotate]: it removed the hard cap on context size, not on session count [inferred from tag; full line not read]. -> no conflict; DECISIONS:16/46 stay valid.
- add new DECISIONS entry (session count cap is a new user decision): "[sessions] 261030 user: 기본 4세션(main opus, tui haiku, coder haiku, prompt haiku) + coder 대기 4 = 최대 8. tui/prompt 분리 필수." note: "does not supersede DECISIONS:16 (context cap)". if the full line 16 turns out to be about session count, mark 16 "superseded by 261030 [sessions]" instead.

## ordered steps (implementer: haiku-coder per file, sonnet-analyst for pickChild/coderpool)
1. internal/harness/children.go
   - add `type Role string`; consts RoleMain="main", RoleTUI="tui", RoleCoder="coder", RolePrompt="prompt".
   - change `func childKey(cwd, model string) string` -> `func childKey(cwd, model string, role Role, slot int) string` (format in D1).
   - liveChild: add fields `Role Role`, `Slot int`.
   - update all callers (grep childKey().
2. internal/harness/dispatch.go
   - DefaultMaxWorkers=3 -> `const DefaultCoderCap = 5`; resolveMaxWorkers -> resolveCoderCap (flag > env > default; range 1..5, error text in D2).
   - add `func roleCap(role Role, coderCap int) int` (main 1, tui 1, prompt 1, coder coderCap).
   - pickChild signature -> `pickChild(set *childSet, reg *clientRegistry, role Role, slot int, cwd, model string, coderCap int, now time.Time, deadline time.Duration) childAction`; search warm -> parked within same role only; new only if count(role live) < roleCap; coder at cap -> actQueue; non-coder at cap -> reuse the single existing session (never new).
   - dispatch.go:43 refuse message: keep only for non-coder unexpected overflow; coder path uses actQueue.
3. internal/harness/coderpool.go (new)
   - `type coderTask struct { ID, Kind string; Files, DependsOn []string; Prompt string }`
   - `type coderPool struct { mu sync.Mutex; leases map[string]int /*file->slot*/; queues map[int][]coderTask; fifo []coderTask; cap int }`
   - `func (p *coderPool) Assign(t coderTask, ctxOf func(slot int) int) (slot int, queued bool)` implementing D3 rules 1-5.
   - `func (p *coderPool) Done(slot int, taskID string)` releases leases, pops next from slot queue else from fifo.
4. internal/harness/repl.go:175 and classifier.go:258-285
   - parked map key: model -> childKey(cwd, model, role, slot). only RoleMain uses the direct Haiku switch park path.
   - repl.go:1091 flag help text -> "coder pool cap (1..5), env CHAUDE_MAX_WORKERS".
5. internal/harness/livechild.go:171 parked digest
   - include only events whose source role == resumed session role. main never receives tui/prompt/coder output text; coder results reach main only as the existing handback summary path.
6. internal/harness/orchws.go
   - on expiry for RolePrompt/RoleCoder: call new `rotateRole(role, slot)` = send "/exit", wait 10s, kill if alive, spawn new `claude` with same key; coder queue lives in coderPool (harness), so no handoff.
   - must not call endAllParked("clear") or any /clear path (repl.go:1168).
7. tui role wiring
   - tui session stdout -> TUI renderer only. never written to main stdin, handoff file, or digest.
8. prompt role wiring
   - repeat/recursive request (DECISIONS:44) -> send to prompt session with instruction to call haiku-prompter subagent once per request; returned per-round log -> TUI only.
9. internal/harness/clients.go:24 Role
   - accept legacy "design" -> main, "exec" -> coder on read; write new values main|tui|coder|prompt.
10. notes/DECISIONS.md: append the D5 entry (haiku-batch-worker, verbatim).

## edge cases
- main /model switch: opus main parked + haiku main live -> main role may hold 2 processes (1 live, 1 parked); not counted against coder/tui/prompt.
- coder crash mid-task: lease released, task re-queued at FIFO head once; second failure -> report to TUI, no retry.
- two tasks with no Files info: treat as disjoint (rule 4) but never assign two tasks with empty Files to run in parallel if Kind equal (rule 3 first).
- /exit not honored (stuck child): kill after 10s; slot freed only after process exit confirmed.
- cwd change: new key per cwd; caps count per role across all cwds (total live <= 8).
- LESSONS:3: rotate must not handoff/overwrite a pending import snapshot -> rotateRole for main is unchanged (existing rotate path).

## acceptance checks
- go test ./internal/harness/ -run 'TestChildKey|TestPickChild|TestRoleCap|TestResolveCoderCap|TestCoderPool|TestRotateRole|TestParkedDigestRole' passes.
- TestChildKey: same cwd+model with role tui vs prompt -> different keys; coder slot 1 vs 2 -> different keys.
- TestPickChild: 6th coder request -> actQueue; tui request never returns prompt session.
- TestCoderPool: task touching a leased file -> same slot; disjoint files -> new slot up to 5; queue>5 or ctx>=24000 -> no append.
- TestParkedDigestRole: main digest contains no tui/prompt text.
- TestRotateRole: no call to endAllParked("clear").
- go vet ./internal/harness/ clean; existing tests (drain_test, orch_test, orchws_test, usagepoll_test) still pass.

## must NOT change
- context threshold 150k (DECISIONS:46); -idle-rotate 3599s (DECISIONS:23); /clear semantics (DECISIONS:13, 42); 5h usage cap 60% (DECISIONS:45); research agents stay subagents (DECISIONS:25); hooks main-only (DECISIONS:26); LESSONS:3 import snapshot rule; wsMaxTurns/wsMaxCtx/wsMaxIdle values.

## open (user)
- "대기 슬롯" meaning: this plan uses lazy spawn (not pre-warmed). pre-warm would keep 4 idle haiku sessions alive and pay cache refresh [inferred].
