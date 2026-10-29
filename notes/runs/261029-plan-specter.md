# Plan: haiku-specter (live progress reporter for the main session)
date: 2026-10-29
status: plan only, no code changed, no agent file created
basis:
- notes/runs/261029-analysis-progress-output.md (progress is feasible in memory: elapsed time, last tool use; subagent token progress not collected)
- notes/runs/261029-plan-local-commands.md (local command plan; its open decisions are numbered 1-9 in that file, the brief called them U1-U7)
- notes/runs/261029-diag-background-panel.md (background agents missing from panel; fix not applied)
- notes/DECISIONS.md lines 36, 38-41 (return-summary rule; Haiku main policy)
legend: [F] fact from notes/source | [I] inference | [U] user decision needed

## user design (quoted, Korean)
- haiku-specter = "메인 세션(opus/sonnet)에 붙어 메인의 중간 진행 상황을 사용자에게 보고하는 에이전트."
- "일반 haiku 에이전트는 각 작업을 끝낼 때마다 무엇을 했는지 추가로 보고."
- "명령 실행을 모델 없이 Go로 처리하는 것과 함께 반영."

## core decisions (proposed)
1. specter is NOT a model. It is an in-memory display in the harness (Go, TUI process). Zero model tokens per tick. [I] Reason: a Haiku agent per tick costs tokens and adds latency; the user asked for Go for command execution too. [U] D1 confirms.
2. Regular Haiku agents report "what they did" through the existing result path only (final message, first line shown in UI). No new file, no new channel. [F] DECISIONS 36 already sets the final-message rule.
3. Sequencing: background-panel fix first, then specter display. Local-command work is independent (0 tokens) and can run in parallel. [F] analysis recommends defer-then-do; [I] specter value for subagents depends on the fix.

## (a) what specter is, and what it must not do
Is:
- A status strip/row in the TUI that shows the main session's current activity: running state, elapsed time, last tool use (e.g. `● Read(path)`).
- Read-only view of harness memory.

Must not:
- Read files on behalf of main, or make main read files for progress. [F] Analysis: display reads happen in TUI memory, not model context.
- Add tokens to the main prompt or to any model call. [F] analysis: "do not feed progress to main model (would grow context)."
- Cost a model call per tick. Tick = 8 Hz spinner already calling running() [F tui.go:327, 372-378]; accessor must be lock + copy only.
- Write a status file or poll agents.json. [F] agents.json poll ~21 KB/read, changes only on state transitions; unsuitable.
- Call /usage or `claude -p` for progress. [F] unsuitable.

## (b) how it gets main-session progress
- Source: events the harness already receives from the claude child and renders (assistant text streaming, tool `● Name(arg)` lines). [F] DECISIONS [tui] line 12 (TUI shows these for main). [U] Whether the same event stream carries usable per-tool timing: unverified; Session.Events body not read (analysis open item).
- Store only in memory: `turnStartedAt` (time), `lastToolLine` (string, last tool use). Set on event receipt. No history kept beyond the last line.
- Accessor: `progress() (elapsed time.Duration, lastTool string, running bool)`. Must not copy the full log per tick (analysis: do NOT call snapshot() per tick, copies up to 400 lines).
- Render: one row/line near the status line; updated by existing spinner tick.
- Token progress for main: not in scope. Main token usage already appears per turn (usageLine). [F] DECISIONS [tui] status line shows ctx/threshold.
- Model choice (Opus/Sonnet/Haiku main): see D2.

## (c) what regular Haiku agents report after each task
- Existing path: the agent's final message is returned via the Agent tool result; the TUI shows its first line. [F] analysis line 8.
- Content rule (already in all 11 .claude/agents/*.md, DECISIONS 36): "Final message = only a brief result summary (a few lines, Korean 존댓말: outcome · changed paths · PASS/FAIL) ... full details go to `notes/runs/`, give only the path."
- Proposed addition for "무엇을 했는지": one extra line "did: <what was done, 1 line>" as the first line of the final message. [U] D5 confirms wording and whether it is required for haiku-* only.
- No new output file. Details stay in notes/runs/.
- Haiku agents are async by default now [F diag: Agent tool_use has no run_in_background], so the completion line only reaches the UI after the background-panel fix (see d).

## (d) relation to background-panel fix and local-command plan
Background panel (notes/runs/261029-diag-background-panel.md):
- [F] Background agents are marked Done at launch (drain.go:72-77), so they vanish from the panel. Specter's subagent rows depend on this fix.
- [F] Fix location: drain.go:62-63, :75; treat "Async agent launched" tool_result as background, do not mark Done.
- Order: fix -> commit -> specter subagent rows. [I]
- Main-session progress (b) does not depend on this fix.

Local-command plan (notes/runs/261029-plan-local-commands.md):
- [F] Local commands = 0 model tokens by design. Specter adds no tokens either, so both fit the "no model per tick" rule.
- Dependency: /rotate and /model <name> use a model handoff turn today (its decision 1-2). If specter reports rotation, it reports whatever the handoff does. No specter change needed for either choice.
- Dependency: /agents (its decision 6) would show the same tracker rows as specter. Choose one row source; do not build two. [I]
- Local plan's open decisions 1-9 are NOT answered here. [U] remain open there.
- Name conflict: the local plan notes no agent named haiku-specter exists; nearest is haiku-scout (model haiku, effort high). [F]

## (e) acceptance per step
S1 accessor (in-memory only)
- Test TestSpecterProgressNoIO: run a fake turn with events; assert no file created under state dir and no claude child spawned for the accessor.
- Test TestSpecterAccessorNoCopy: accessor does not call snapshot(); assert via benchmark or code grep that it touches only the last line.
S2 TUI row (main progress)
- Test TestSpecterRowRenders: with running turn and lastTool "● Read(x)", row contains elapsed and "Read(x)"; with idle turn row absent.
- Manual: capture in notes/runs/ with a live run (not yet done).
S3 subagent rows (after background fix)
- Test TestBackgroundAgentShownRunning: Agent tool_use without run_in_background + "Async agent launched" result -> running row present, not Done.
- Test: completion via <task-notification> marks Done.
S4 Haiku final line
- grep: each .claude/agents/haiku-*.md contains the return-summary line (DECISIONS 36) and, if D5 = yes, the "did:" line.
- Check: UI first line of a finished agent equals the "did:" line (test with fixture result).
S5 no-token guarantee (whole feature)
- Count model calls during a 60s idle + 60s running window with specter enabled: expected 0 extra calls. Measure with fake claude that logs argv; assert log has only the real turn's calls.

Each step: go test ./internal/harness/... passes; no new file under notes/ except notes/runs/ entries.

## (f) user decisions (separate list; none answered by this plan)
- D1. specter = Go in-memory display (recommended, 0 tokens) OR a Haiku agent that polls (costs tokens per tick; not recommended). [U]
- D2. Main models specter attaches to: brief says opus/sonnet, but DECISIONS 38-41 make Haiku the main default (Opus only for design; Haiku rotates at 80k). Confirm: all mains, or only Opus/Sonnet? [U]
- D3. Name: keep "haiku-specter" (only if it is a model-backed agent file), or use a non-agent name since D1 is Go-only? [U]
- D4. Progress content: elapsed + last tool line only (analysis recommendation), no token progress. Confirm. [U]
- D5. Haiku final line: add a "did: <1 line>" first line for haiku-* agents (brief asks "무엇을 했는지"), or keep the existing summary only? [U]
- D6. Order: background-panel fix first, then specter subagent rows (analysis recommendation)? Or specter first? [U]

Count of user decisions: 6 (D1-D6). Local-command decisions 1-9 are a separate list in its own file.

## open / uncertain
- Session.Events body not read; per-tool timing and whether main events carry usable usage: unverified.
- Background-panel diagnosis not runtime-verified (no live TUI run).
- "elapsed" is inference from started field (agentstore.go:28-43); main-turn start time source not verified.
- No file in repo changed by this plan.
