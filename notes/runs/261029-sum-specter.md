# Summary: haiku-specter plan + review (terse)
date: 2026-10-29
sources:
- /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261029-plan-specter.md (94 lines)
- /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261029-plan-specter-review.md (50 lines, reviewer opus-architect)
legend: [F] fact from source | [I] inference | [U] user decision | [R] review edit
status: plan only. no code changed. no agent file created.

## user brief (verbatim, Korean)
- "메인 세션(opus/sonnet)에 붙어 메인의 중간 진행 상황을 사용자에게 보고하는 에이전트."
- "일반 haiku 에이전트는 각 작업을 끝낼 때마다 무엇을 했는지 추가로 보고."
- "명령 실행을 모델 없이 Go로 처리하는 것과 함께 반영."

## core decisions (plan)
1. specter = NOT a model. In-memory display in harness (Go, TUI). 0 model tokens per tick. [U] D1.
2. Regular haiku agents report "what they did" via existing result path only. No new file/channel. DECISIONS 36 final-message rule.
3. Order: background-panel fix first, then specter subagent rows. Local-command work independent (0 tokens), can run in parallel.

## (a) scope
Is: TUI status row, read-only view of harness memory (running state, elapsed, last tool e.g. `● Read(path)`).
Must not (plan):
- read files for main or make main read files for progress
- add tokens to main prompt or any model call
- cost a model call per tick (tick = 8 Hz spinner, tui.go:327, 372-378; accessor = lock + copy only)
- write status file or poll agents.json (~21 KB/read)
- call /usage or `claude -p` for progress
[R5] add: must not open notes/runs/* or agent OutPath to build a row (path string only); must not inject progress into any prompt, handoff snapshot (buildSnapshot repl.go:536), or classifier input.

## (b) main-session progress (plan, then revised by [R1])
Plan: store in memory `turnStartedAt`, `lastToolLine`; accessor `progress() (elapsed, lastTool, running)`; do not call snapshot() per tick (copies up to 400 lines); one row near status line, updated by spinner tick.
[R1] revised (F): main data already in TUI. tui.go:310-314 renders spinner + mainStatusText() = `생각중-<model>-메인(<prompt 60자>)` while busy; tool lines already arrive as toolMsg (tui.go:384-391, name+arg).
  - extend mainStatusText() (tui.go:727) to `생각중-<model>-메인(<summary>) · <elapsed> · <lastTool>`
  - tuiModel fields: turnStart time.Time, lastTool string
  - set turnStart=time.Now(), lastTool="" at every `m.busy = true` site: tui.go:531, 709 (sendPrompt), 884, 900
  - set lastTool in `case toolMsg` (tui.go:384)
  - no accessor on Repl/Session for main. Delete the "Session.Events body unverified" open item for main.

## (c) regular haiku agents report
Plan: final message returned via Agent tool result; content rule already in all 11 .claude/agents/*.md (DECISIONS 36): "Final message = only a brief result summary (a few lines, Korean 존댓말: outcome · changed paths · PASS/FAIL) ... full details go to `notes/runs/`, give only the path."
Plan proposal: add first line `did: <what was done, 1 line>` for haiku-* (see D5). No new output file.
[R2] (F) WRONG in plan: UI summary = LAST plain-text line of streamed log, not first line. agentview.go:246-247 `a.Summary = lastSummary(a.Log)`; agentstore.go:216 "last plain text line". onNotification (drain.go:84-99) only sets Done, does not read result text. Async agents may have no Log lines at all.
[R3] Per-task return enters main context in full (foreground tool_result / async <task-notification>). Main restating it = duplicate output tokens. Haiku main rotates at 80k (DECISIONS 41).
  - did: must REPLACE the outcome field of existing summary, not add a line.
  - cap: final message <= 3 lines and <= 300 chars total (Korean ~200-300 tokens per agent return); details only via notes/runs path.

## (d) dependencies
- Background panel (notes/runs/261029-diag-background-panel.md): [F] background agents marked Done at launch (drain.go:72-77) so they vanish from panel. Fix location drain.go:62-63, :75; treat "Async agent launched" tool_result as background, not Done. Order: fix -> commit -> specter subagent rows [I]. Main progress (b) does not depend on fix.
- Local-command plan (notes/runs/261029-plan-local-commands.md): local commands = 0 tokens. Specter adds 0 tokens. /rotate and /model <name> use model handoff turn (its decision 1-2); specter reports whatever happens, no change needed.
- /agents (local plan decision 6) shows same rows as specter. One row source only. [R8] shared source: tracker.running() + new lastTool(id) + agentStatusText (tui.go:1232); /agents and specter both call agentStatusText; no second formatter.
- Name conflict: no haiku-specter agent exists; nearest is haiku-scout (model haiku, effort high). [F]

## (e) steps and tests (plan)
- S1 accessor (in-memory only)
  - TestSpecterProgressNoIO: fake turn with events; assert no file under state dir, no claude child spawned for accessor.
  - TestSpecterAccessorNoCopy: accessor does not call snapshot(); check via benchmark or grep; touches only last line.
- S2 TUI row (main progress)
  - TestSpecterRowRenders: running turn + lastTool "● Read(x)" -> row contains elapsed and "Read(x)"; idle turn -> row absent.
  - Manual: capture in notes/runs/ with live run (NOT yet done).
- S3 subagent rows (after background fix)
  - TestBackgroundAgentShownRunning: Agent tool_use without run_in_background + "Async agent launched" result -> running row present, not Done.
  - completion via <task-notification> marks Done.
- S4 haiku final line
  - grep: each .claude/agents/haiku-*.md has return-summary line (DECISIONS 36) and, if D5=yes, "did:" line.
  - UI first line of finished agent == did line (fixture). [R2] corrected: Summary of finished agent (fixture: foreground tool_result AND <task-notification>) == did line.
- S5 no-token guarantee
  - plan: 60s idle + 60s running with fake claude logging argv; expect 0 extra calls.
  - [R10] replace with unit check: fake PATH claude logs argv; 100 spinner ticks via Update(spinner.TickMsg{}); assert log empty.
- every step: go test ./internal/harness/... passes; no new file under notes/ except notes/runs/.

## review findings (all 10, plus verdict)
verdict: direction right (Go in-memory, 0 tokens, fix first). 2 [F] errors in plan (items 1, 2) change S2/S4 specs.
code read: tui.go:305-314, 365-391, 528-531, 706-709, 725-736, 881-900, 1200-1235; agentview.go:15-66, 246-247; drain.go:71-100; grep Summary/onNotification in repl.go.
1. [F] main progress row already exists; S2 = extend it, not new row. (plan (b) edit above)
2. [F] agent summary = last plain line of Log (agentview.go:246-247); async may have none. Options: (A) put did content as LAST line of final message; (B) code: onNotification/onToolResult extract result text, set Summary from first line. Review recommends B for async agents; keep line order free.
3. per-task report adds tokens to main context; cap it; do not duplicate. did replaces outcome field; cap <=3 lines, <=300 chars; add D7; add S4 grep for 300-char cap in each haiku-*.md.
4. [U/F] subagent last-tool for async agents unverified (onSubEvent agentview.go:19-66 only logs events with parent_tool_use_id from main child stream). Add S3a: live run (or transcript 17d8f798) to check whether async events carry parent_tool_use_id. If no: subagent row = state + elapsed only. Tailing OutPath = harness I/O not tokens but violates no-poll rule -> D8 default no. Accessor spec: `func (t *agentTracker) lastTool(id string) string` (lock, scan Log backwards for first "● " line, return; no Log copy). Elapsed = time.Since(a.Started).
5. consistency with DECISIONS 38-41 (main reads no extra files, Haiku main default): satisfied; guard = no open/render of notes/runs/* or OutPath; path string only; no injection into prompt, buildSnapshot (repl.go:536), classifier.
6. [U] D2 reframe: under Haiku main default, opus/sonnet "main" mostly = opus-architect/sonnet-analyst SUBAGENTS, so specter's opus/sonnet mapping = subagent rows (S3, depends on fix). Main row (item 1) is model-agnostic, 0 cost -> show for all mains (technical no-cost default). Real user choice = subagent-row scope. Reword D2 accordingly.
7. ordering: split background fix into own step; clear dirty tree first. New order: S0 = commit or confirm current tui.go/repl.go edits ([U] user decides commit); S1 = background fix alone (drain.go:62-63, :72-77 "Async agent launched" -> Background=true, not Done; finishOldestBackground covers them; onNotification sets Summary per item 2) + tests, commit; S2 = main row (item 1), commit; S3 = subagent rows (item 4), commit; S4 = agent-file wording (item 3). S2 may run before S1 (no dependency), state it. Note: git status shows tui.go, repl.go modified (uncommitted, 0.7.1 follow-up).
8. one row source with /agents (see (d) [R8]).
9. naming: "specter" is not an agent; "haiku-specter" implies .claude/agents file and haiku-* route target (hook allows haiku-*). If D1=Go: code name non-agent (e.g. `progressRow`), no agent file. Review recommends D3 = no agent file.
10. delegation + acceptance: add `## [haiku] steps`: S1 haiku-coder (drain.go + drain_test.go, spec from diag file + item 2); S2 haiku-coder (tui.go fields/mainStatusText per item 1, TestSpecterRowRenders); S3 haiku-coder after S3a result; S4 haiku-batch-worker (11 agent files, identical cap sentence). Verify each: go test ./internal/harness/... . S5 replaced per [R10]. Elapsed format `<m>m<ss>s` under 1h (e.g. `2m05s`); lastTool truncated with trunc(…, 40).

## open decisions (plan section f + review additions)
Plan default column: "rec" = recommended in source; "none" = plan gives no default.
- D1. specter = Go in-memory display (rec, 0 tokens) OR haiku agent that polls (costs tokens per tick; not rec). rec: Go.
- D2. main models specter attaches to: brief says opus/sonnet; DECISIONS 38-41 make Haiku main default. Plan: "all mains, or only Opus/Sonnet?" default: none in plan. Review [R6]: show main row for all mains (0 cost, rec); user choice = subagent-row scope (opus-architect/sonnet-analyst only, or all agents).
- D3. name: keep "haiku-specter" (only if model-backed agent file) or non-agent name. default: none in plan. Review [R9] rec: no agent file, code name progressRow, user-facing label only if wanted.
- D4. progress content: elapsed + last tool line only, no token progress. rec: yes (analysis recommendation; review agrees).
- D5. haiku final line: add `did: <1 line>` first line for haiku-* only (brief asks 무엇을 했는지), or keep existing summary only. Plan proposal: add for haiku-* only. Review [R3]: did REPLACES outcome field; cap <=3 lines, <=300 chars. Final-line position = plan says first line, review [R2] says Summary is LAST line -> choose option A (last line) or B (code change).
- D6. order: background fix first, then subagent rows (analysis rec) vs specter first. rec: fix first. Review [R7] adds: fix as own step S1.
- D7. [R3, review-added, no default in plan] harness shows agent summary directly (Summary row/notice); main does NOT restate it. Amends DECISIONS 36 ("main posts it"). Saves duplicate output tokens. Default: none stated.
- D8. [R4, review-added] tail OutPath fallback for async subagent progress. default: no (per review and no-poll rule).
- S0. [R7, review-added] commit current dirty tui.go/repl.go edits (0.7.1 follow-up) before any step. [U] user decides commit. default: none stated.

## not found / open (plan + review)
- Session.Events body not read; per-tool timing for main: moot per [R1] (TUI already has toolMsg).
- Async agents: whether events carry parent_tool_use_id in main stream-json: unverified (S3a).
- Background-panel diagnosis not runtime-verified (no live TUI run).
- "elapsed" source: inference from started field (agentstore.go:28-43); main turn start time source not verified (plan), resolved by [R1] turnStart set at m.busy sites.
- manual capture of live run for S2: not done.
- no file in repo changed by plan.
