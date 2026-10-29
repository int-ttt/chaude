# review: 261029-plan-three-session-structure.md
reviewer: opus-architect
date: 261029
scope: improvements only; plan and source not edited
verdict: structure follows the 6 user points, but (a) has no routing rule, (b) is not decidable from the data Opus gets, and (e) misses 6 DECISIONS rows. Apply items 1-6 before implementation.

## code facts checked (working tree)
- classifier.go:15-23 classifyPrompt outputs topic/size/kind/worker/task only; no `model:` line
- classifier.go:39-52 pickModel/pickTarget always return haiku (DECISIONS:38); forced opus/sonnet kept
- classifier.go:32-34 planHint = Haiku writes plan, Opus/Sonnet review; tested by TestHaikuPlanWorkflow
- classifier.go:14 conductorModel = "sonnet" (classifier model, verified live DECISIONS:33)
- classifier.go:183-210 + repl.go:157 `parked` map = model -> session id; haikupool.go workerPool = Haiku sessions with edit history (max 5 candidates, in memory)
- repl.go:26-27 opusSonnetThreshold=270_000; repl.go:870 -threshold default 80_000 (haiku); repl.go:695 effectiveThreshold uses only the single current r.a.Model
- repl.go:871 child compact window 400_000 (> 270k, ok)
- repl.go:755-765 rotation after each answer: category switch first, then ctx >= threshold
- idle rotation (DECISIONS:19/21/23): no tea.Tick/lastInput/idle-rotate in non-test code = not implemented
- Repl holds one live r.sess + parked ids + draining; no concurrent second live session

## improvements

1. (a) 분류자 흐름: add an explicit model rule. Where: "분류자 흐름", after 점2.
   - change: classifier must emit a `model: opus|sonnet|haiku` line, and the plan must state the mapping so it is testable. Proposal from existing tags:
     - kind=lookup or edit, size=small -> haiku
     - size=large, kind=other, review/analysis/comparison of given material -> sonnet (DECISIONS:35 sonnet=검토·분석)
     - size=large with design/architecture/strategy/trade-off/correction/pushback/'opus로'/repeated failure -> opus (DECISIONS:30, :33 signal list)
     - missing/invalid model line -> derive from size/kind by the rule above (same safe-side pattern as size default large)
   - also state: pickModel/pickTarget stop being Haiku-fixed; forced --model still wins.
   - open Q1 (classifier sonnet vs haiku): propose default = sonnet (only one verified live, DECISIONS:33); haiku classifier only after TestClassifierLive passes on haiku. Mark as 사용자 확인 필요, not 가정.

2. (a) 세션 구성: define what "opus 실행 세션" is. Where: "세션 구성" bullet 2 and open Q3.
   - problem: with an always-on Opus 설계 세션, a classifier pick of opus means either a 2nd Opus context (double Opus history billing) or the design session doing the work.
   - change: add rule "model=opus -> the task runs in the Opus 설계 세션 itself (no separate Opus exec session)" as the proposal, flagged 사용자 확인 필요. This also answers Q3 (opus is never subject to keep/end; it is the permanent session).

3. (b) 유지/종료 규칙: specify the input payload Opus gets, else the 1h judgment is not decidable. Where: "유지/종료 규칙", new bullet after 점3; and "데이터 흐름" step 2.
   - problem: per 점4 Opus receives only classifier prompt + answer per user prompt. classify() input today = current model, previous prompt (200 chars), Haiku candidates. No session list, no timestamps, no category, no ctx. Opus cannot predict "1시간 안에 비슷한 작업" from that.
   - change: chaude attaches a session table to each forward: `id · model · task(terse) · cat label (DECISIONS:29) · files (workerPool) · last_used (RFC3339) · ctx · running agents`, plus the timestamp of each classifier event.
   - define "비슷한 작업" = same `[cat: X]` label or file overlap with workerPool entry (reuse existing data; no new similarity model).
   - define Opus output as machine lines only, parsed by chaude and not shown to the user: `keep: <id>` / `end: <id>` (one per live sonnet/haiku session). Parse failure or no answer -> default end at expiry (item 4).

4. (b) 유지/종료 규칙: fix the 1h clock and the expiry action; resolves open Q4. Where: "유지/종료 규칙" bullet 3 (가정 "1시간") and bullet 4 (충돌 DECISIONS:19).
   - DECISIONS:19 (15m) is superseded by :21 (4m50s) and :23 (cache TTL pinned 1h, idle handoff 59:59, recommend 3570s). Replace the :19 citation with :23.
   - proposal: keep window = last_used + 3570s (same as cache TTL, so a kept session is still cache-warm when reused). At expiry with no similar task: end without asking Opus again (snapshot via normal handoff, or no handoff if the parked id is enough for resume). Rationale: after TTL the kept session's next turn pays full cache write anyway, so keeping longer than TTL saves nothing over a new session + handoff.
   - note: idle rotation is not implemented in code yet; the plan must not assume it exists.

5. (b) cost of 점4: say how classifier I/O reaches Opus without an Opus turn per user prompt. Where: "분류자 흐름" 가정 line 24 and open Q2.
   - problem: in stream-json every message sent to the Opus session starts an Opus turn; forwarding every classifier exchange = 1 Opus turn per user prompt, growing Opus context toward 270k fast. Conflicts with DECISIONS:40 (1) "Opus main only for design turns".
   - change: chaude appends each classifier exchange + session table to a log file (state dir or `notes/runs/<YYMMDD>-route-log.jsonl`); Opus reads it only (i) on a design turn (model=opus or size=large) and (ii) at a keep/end decision point (a kept session reaches expiry, or a new sonnet/haiku session would be started while others are live). Small haiku routes do not wake Opus.
   - Q2 order: for model=opus/large tasks the exec session waits for the design file (item 7); small tasks never wait.

6. (c) 컨텍스트 한도: the "일치" line is wrong; fix and add the at-limit behavior per session. Where: "컨텍스트 한도" bullets 2-3.
   - conflict missed: DECISIONS:40 (2) says Opus/Sonnet mains do not switch by threshold; :41 and code (repl.go:27, :695) rotate them at 270k. Write "충돌: :40(2) vs :41; 채택 :41 (user correction, matches code)" and propose superseding :40(2) in DECISIONS.
   - 150k/100k are not session-rotation numbers: CLAUDE.md:27 150k = plain `claude` auto-compact; CLAUDE.md:19 100k = subagent compact. In chaude children compact window = 400k (repl.go:871) > 270k, so 270k rotation fires first. 300k in DECISIONS:14 is superseded by :16. Replace open Q6 with this statement; nothing to tidy except noting 400k must stay > 270k.
   - at the limit (add per session): after the answer, ctx >= limit -> rotate() with handoff (repl.go:764); running background agents -> draining (DECISIONS:18, max 3); new session id inherits the keep/end state and the workerPool/parked entry of the old id.
   - Opus 설계 세션 at 270k: handoff can be short = list of its design file paths + open items, since design state is on disk.
   - implementation note: effectiveThreshold reads only r.a.Model; with 2 live sessions each needs its own threshold and lastCtx.

7. (d) 데이터 흐름: remove the duplicated fetches. Where: "데이터 흐름" steps 2-4.
   - dup 1 (race): with 병행 start (가정 line 24) the exec session may start before the design file exists and fetch sources itself -> Opus and exec both read the same sources. Change: chaude gives the exec session the exact design path (chaude picks `notes/runs/<YYMMDD>-plan-<topic>.md`, topic from classifier task) and starts it only when the file is done (reuse drain rule: exists + size>0 + stable 2s, DECISIONS:18).
   - dup 2 (re-search): require a fixed section in the design file, `## evidence` with `path:line` refs and the facts Opus already read; haiku-scout reads only that section and fetches only what is missing. No scout re-search of sources Opus listed.
   - dup 3 (extra hop): require `## brief` (<= 3,000 chars for haiku, <= 6,000 for sonnet, same caps as DECISIONS:41 overview) that chaude pastes verbatim into the exec task; haiku-scout is called only when the exec needs more than the brief. A Haiku exec session may read the one design file directly (one file, no scout hop); the no-direct-read rule stays only for sonnet-analyst (DECISIONS:39, max 3 files/group).
   - dup 4: Opus already gets the user prompt inside the classifier input; state that chaude does not resend it separately.

8. (e) 충돌 table: add the DECISIONS rows the plan does not cover. Where: new subsection "충돌 정리" before "열린 질문".
   - :38 code impact: planHint, pickModel Haiku-fixed, TestHaikuPlanWorkflow, CLAUDE.md:31, opus-architect/sonnet-analyst "Plan review" paragraph -> mark each as superseded or kept (proposal: planHint kept only when classifier says haiku+large without Opus design session available; else removed).
   - :29 category rotation: state which sessions it applies to (proposal: exec sessions yes; Opus 설계 세션 no, it spans categories) and that cat labels feed item 3.
   - :24-27 session workers vs subagents: exec sessions are the "worker session" option; state that research stays subagents (:25, already kept) and that the conductor's subagent-vs-worker choice (:27) is replaced by Opus keep/end.
   - :28 main.md / main hook set: say which session gets main hooks (proposal: Opus 설계 세션 = main set; exec sessions = worker set).
   - :30 who writes the user reply: exec session writes the Korean 존댓말 reply; Opus design-session output (keep/end lines, design file notices) is not shown to the user. Say this in "세션 구성".
   - existing mechanisms: build keep/end on `parked` + workerPool (classifier.go:183-210, haikupool.go) instead of a new registry; say so.

9. 수용 확인 (missing section). Where: add "## 확인 항목" before "열린 질문".
   - classifier model line: TestClassifierLive extended with 3 sonnet-target prompts (review/analysis) and the existing 6 (opus 4, haiku 2).
   - metrics: new stage=keepend {session, model, decision, reason, expires_at}; stage=classify gains routed_model for all 3 models.
   - check: small haiku prompt does not create an Opus turn (count Opus turns in metrics = design turns + decision points only).
   - check: exec session for a large task starts after the design file exists (timestamps: file mtime < exec first turn start_ms).

10. 열린 질문 cleanup. Where: "열린 질문".
   - Q4 and Q6 become answered proposals by items 4 and 6 (keep as 사용자 확인 필요 only for the 3570s value).
   - add: Q-new "model=opus 시 설계 세션이 직접 수행하는가, 별도 Opus 실행 세션인가" (item 2).
   - add: Q-new "Opus 설계 세션이 1시간 이상 쉬면 캐시 갱신(59:59 핑)으로 유지하는가, 만료 후 다음 설계 턴에서 다시 읽는가" (always-on Opus has an hourly cache-refresh cost under DECISIONS:23).

## ordering for implementation (after user answers)
1 classifier model line + mapping (items 1-2) -> 2 route log file + session table (items 3, 5) -> 3 design-file wait + brief/evidence sections (item 7) -> 4 keep/end parser on parked/workerPool with 3570s expiry (item 4) -> 5 per-session thresholds (item 6) -> 6 checks (item 9). Steps 1, 4, 5 are bounded code changes for haiku-coder; 2-3 touch Repl session ownership (two live sessions) and need a design pass first.
