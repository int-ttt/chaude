# Review: 261029-plan-specter.md
date: 2026-10-29
reviewer: opus-architect (plan review only; no source/CLAUDE.md/DECISIONS edited)
verdict: direction right (Go in-memory, 0 tokens, fix first). 10 improvements; 2 [F] errors in the plan (items 1, 2) change S2/S4 specs.
code read: tui.go:305-314, 365-391, 528-531, 706-709, 725-736, 881-900, 1200-1235; agentview.go:15-66, 246-247; drain.go:71-100; grep Summary/onNotification in repl.go.

## improvements (each with plan edit)

1. [F] Main progress row already exists; S2 = extend it, not a new row.
   - fact: tui.go:310-314 renders `spinner + mainStatusText()` (`생각중-<model>-메인(<prompt 60자>)`) while busy; tool lines already arrive as toolMsg (tui.go:384-391, name+arg). So the "Session.Events body not read / per-tool timing unverified" open item is moot for main: the TUI already has the data.
   - plan edit (b): replace "Render: one row/line near the status line" with: "extend mainStatusText() (tui.go:727) to `생각중-<model>-메인(<summary>) · <elapsed> · <lastTool>`; add tuiModel fields `turnStart time.Time`, `lastTool string`; set turnStart=time.Now(), lastTool="" at every `m.busy = true` site (tui.go:531, 709 sendPrompt, 884, 900); set lastTool in `case toolMsg` (tui.go:384). No accessor on Repl/Session needed." Drop `progress()` accessor for main; delete the open item about Session.Events for main. S1 test TestSpecterProgressNoIO then only covers the subagent accessor (item 4).

2. [F] Agent summary in UI is the LAST plain-text line of the streamed log, not the first line; async agents may have none.
   - fact: agentview.go:246-247 `a.Summary = lastSummary(a.Log)`; agentstore.go:216 "last plain text line". onNotification (drain.go:84-99) only sets Done, does not read the notification's result text. The plan's (c) "[F] TUI shows its first line" and S4 "UI first line equals did: line" are wrong.
   - plan edit (c)/S4: either (A) put the "did:" content as the LAST line of the final message, or (B) add code: onNotification/onToolResult extract the result text and set Summary from its first line. Recommend (B) for async agents (their sub-events may not stream into Log at all, see item 4) and keep line order free. Change S4 check to "Summary of a finished agent (fixture: foreground tool_result AND <task-notification>) equals the did line".

3. Per-task report does add tokens to the main context; cap it and do not duplicate.
   - fact: the Agent tool_result (foreground) / <task-notification> (async) body enters the main model context in full; the TUI first/last line is only a display. DECISIONS 36 already says main "posts" the summary -> main re-emits it as output tokens too (counted twice). [I] Haiku main rotates at 80k (DECISIONS 41), so per-agent overhead matters there most.
   - plan edit (c): "did:" must REPLACE the outcome field of the existing summary, not add a line. Cap: final message <= 3 lines and <= 300 chars total (Korean ~1 token/1-2 chars -> roughly <=200-300 tokens per agent return); details only by notes/runs path. Add a [U] decision D7: "harness shows the agent summary directly (Summary row/notice) and main does NOT restate it" (would amend DECISIONS 36 'main posts it'; saves the duplicate output tokens). Add S4 grep: each haiku-*.md states the 300-char cap.

4. [U/F] Subagent last-tool for async agents is unverified; give a fallback.
   - fact: onSubEvent (agentview.go:19-66) only logs events with parent_tool_use_id from the main child's stream. [I/unverified] async (default) agents run detached; their events may not arrive in the main stream at all, so Log stays empty -> no last-tool, no Summary.
   - plan edit: add step S3a "verify with a live run (or transcript 17d8f798) whether async agent events carry parent_tool_use_id in main's stream-json". If no: subagent row = state + elapsed only (no file tail). Tailing OutPath is harness I/O (not model tokens) but violates the plan's "no poll" rule -> list as [U] D8, default no.
   - add accessor spec: `func (t *agentTracker) lastTool(id string) string` (lock, scan Log backwards for first "● " line, return it; no copy of Log). Elapsed = time.Since(a.Started).

5. Consistency with DECISIONS 38-41 (main reads no extra files, Haiku main default): state it explicitly as satisfied, plus one guard.
   - fact/inference: specter is TUI process memory; no model reads anything; no tokens. Consistent. Guard: specter must not render notes/runs/* content or open OutPath; it shows only the path string if any.
   - plan edit (a) Must-not: add "must not open notes/runs/* or agent OutPath to build a row (path string only)" and "must not inject progress into any prompt, handoff snapshot (buildSnapshot repl.go:536), or classifier input".

6. [U] D2 reframe: under the default Haiku main, "opus/sonnet main" is mostly opus-architect/sonnet-analyst SUBAGENTS.
   - [I] DECISIONS 38/40: main is Haiku; Opus runs only for design turns, usually via opus-architect/sonnet-analyst. So the user's "specter on opus/sonnet" maps mostly to subagent rows (S3), which depend on the background fix. The main-row extension (item 1) is model-agnostic and costs 0, so showing it for all mains is a technical no-cost default, not a trade-off.
   - plan edit (f) D2: reword to "Show progress (a) for the main row on every main model (0 cost, recommended) and (b) for subagent rows of opus-architect/sonnet-analyst only, or all agents?" The real user choice is (b) scope.

7. Ordering: split the background fix into its own step 0 and commit; also clear the dirty tree first.
   - fact: git status shows tui.go, repl.go modified (uncommitted, 0.7.1 follow-up). Plan S3 mixes the fix test (TestBackgroundAgentShownRunning) into the specter step.
   - plan edit (d)/(e): new order: S0 = commit or confirm current tui.go/repl.go edits ([U]: user decides commit); S1 = background fix alone (drain.go:62-63, :72-77 "Async agent launched" -> Background=true, not Done; finishOldestBackground covers them; onNotification sets Summary per item 2) + its tests, commit; S2 = main row (item 1), commit; S3 = subagent rows (item 4), commit; S4 = agent-file wording (item 3). S2 can run before S1 if desired (no dependency) - keep it stated.

8. One row source with /agents (local-commands decision 6).
   - plan edit (d): name the shared source: tracker.running() + new lastTool(id) + agentStatusText (tui.go:1232). /agents local output and specter rows must both call agentStatusText; no second formatter.

9. Naming: "specter" is not an agent; avoid confusing the agent-model-pin hook / routing.
   - [I] a name "haiku-specter" implies a .claude/agents file and a haiku-* route target (hook allows haiku-* calls). If D1 = Go, the feature name in code should be non-agent (e.g. `progressRow`), and no agent file. plan edit (f) D3: recommend "no agent file; code name progressRow; user-facing label only if user wants one".

10. Delegation and acceptance details missing.
   - plan edit: add `## [haiku] steps`: S1 haiku-coder (drain.go + drain_test.go, spec from diag file + item 2), S2 haiku-coder (tui.go fields/mainStatusText per item 1, test TestSpecterRowRenders), S3 haiku-coder after S3a result, S4 haiku-batch-worker (11 agent files, identical cap sentence). Verification after each: `go test ./internal/harness/...`.
   - S5 fix: the "60s idle + 60s running" fake-claude test is heavy; replace with unit assertion that render/accessor code paths call no exec (fake PATH claude logs argv; run 100 spinner ticks via Update(spinner.TickMsg{}); assert log empty). Elapsed format spec: `<m>m<ss>s` under 1h, e.g. `2m05s`; lastTool truncated with trunc(…, 40).

## user decisions vs technical facts (for plan (f))
technical (no user input needed): 0-token Go display is possible (items 1, 4); main row data already in TUI (item 1); Summary source is last log line (item 2); agent returns enter main context (item 3); background fix is a prerequisite for subagent rows (item 7).
user: D1 Go vs agent (recommend Go); D2 subagent-row scope (item 6); D3 name (item 9); D4 content elapsed+last tool (recommend yes); D5 did-line replaces outcome + 300-char cap (item 3); D6 order (item 7, recommend fix first); D7 main does not restate agent summary (amends DECISIONS 36); D8 tail OutPath fallback (default no); S0 commit current dirty edits.
