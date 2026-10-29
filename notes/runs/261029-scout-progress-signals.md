# scout: progress signals for Haiku/subagent work (read-only)

date: 2026-10-29
scope: internal/harness working tree (tui.go, repl.go modified in git status); no runtime run, code reading only
status: done. no files edited except this note.

## (1) how agent results return to the main session
- main turn: runTurn repl.go:388-502. Events read via Session.Events (repl.go:484) -> handle() (repl.go:408).
- main reply is STREAMED: text_delta -> emit (repl.go:414-427) -> textMsg (tui.go:379).
- sub-agent output is STREAMED into memory only: onSubEvent agentview.go:19-66 appends text / "● tool(args)" / "  ⎿ result" lines to agentRec.Log (cap maxAgentLog=400, agentview.go:13). Called at repl.go:410 (turn) and repl.go:212 (idle).
- final sub-agent result is COMPLETION-ONLY into main: tool_result of Agent call -> onToolResult drain.go:72-79 sets Done only when !Background (drain.go:75). Tool result shown in UI as first line only (repl.go:469-473, tui.go:392-400).
- background agents: launch result returns immediately; completion via <task-notification> (onNotification drain.go:84-100; repl.go:247, 256) or fallback finishOldestBackground drain.go:116 (repl.go:309).

## (2) live status / token / progress writes during a run
- status file: NONE. no progress/token file written during a run.
- ~/.chaude/agents.json: default path agentstore.go:250-256 (flag repl.go:876). Written by persistLocked agentview.go:230-252 -> store.put/flushLocked agentstore.go:71-172 (full rewrite via .tmp + rename, max 200 rows, 7-day prune).
  - write triggers: onToolUse drain.go:68, onToolResult drain.go:77 (foreground only), onNotification drain.go:97, finishForeground drain.go:111, finishOldestBackground drain.go:127, interruptRunning agentview.go:279.
  - NOT triggered by onSubEvent, so sub-agent progress lines and usage are never persisted.
  - fields: id, model, name, task, prompt, status (running|done|failed|interrupted), summary, parent_session, transcript, out_path, background, owner_pid, started, updated (agentstore.go:28-43). No token or progress counter.
  - each put re-reads the whole file first (mergeDiskLocked agentstore.go:123-137).
- metrics: notes/metrics/<date>.jsonl via Metrics.Log metrics.go:22-47. Written at turn end only: repl.go:335 (background turn), repl.go:741 (main turn), classifier.go:155,163, rotate repl.go:648, category switch repl.go:744. Carries usage tokens (usageFields metrics.go:50-58), cost, ctx. Not live.
- transcript: <state>/transcript.jsonl (repl.go:863, 889; Transcript.add repl.go:85-98). Append per event kind. assistant usage written at turn end (repl.go:732).
- LEDGER.md: no Go code writes it (no refs in internal/). Written by .claude/hooks/ledger.py and by agents by hand. Not live.
- live tokens: in memory only. lastUsage repl.go:406, 430-431; idle.usage repl.go:229-230; r.lastCtx read by statusLine tui.go:958. onSubEvent ignores the usage field of sub-agent assistant events (agentview.go:26-41), so per-sub-agent tokens are not captured at all.
- in-memory values the UI already reads (under tracker mutex t.mu): running() drain.go:131-141, counts() drain.go:176-185, sortedRunning() drain.go:197-206, snapshot() agentview.go:69-79 (copies Log). Panel rows tui.go:1164-1199; status line "agents N running (bg M)" tui.go:957-978; main row "생각중" tui.go:728-736.
- existing UI tick: spinner FPS = time.Second/8 = 125 ms (tui.go:215). On each tick, refresh() runs if busy or running count changed (tui.go:372-378). refresh() also calls running() (tui.go:327).

## (3) read cost if the UI polls a value
- in-memory tracker field: a mutex lock plus slice copy of agent records. No I/O. The 8 Hz tick already does this, so a new poll of the same kind adds no new I/O. snapshot() copies up to 400 Log lines per call, so keep it out of the tick.
- agents.json as the poll source: each read = whole-file read plus json parse. Current size 20,953 bytes, 375 lines, 29 entries (stat 11:38:58). At 8 Hz that is about 164 KiB/s of reads. Also it only changes on state transitions, so it cannot show progress.
- notes/metrics/2026-10-29.jsonl as poll source: 69,428 bytes (stat 11:38:33), grows per turn, per-turn values only. Not suitable for live polling.
- /usage is the only path that spawns a process: usageFetch runs `claude -p /usage` (tui.go:691-702). Do not poll it.

## background-panel state (notes/runs/261029-diag-background-panel.md)
- diagnosis: real Agent tool_use no longer carries run_in_background. Tracker reads flag at drain.go:57, sets Background=false at drain.go:63. onToolResult then marks Done on the "Async agent launched" result (drain.go:75). running() drops it, so rows() (tui.go:1164-1199) shows nothing in either panel.
- fix: NOT applied. Note proposes detecting "Async agent launched" in onToolResult and setting Background=true without Done; plus a no-flag test case. Not reproduced at runtime.
- line refs in the note are slightly off: flag read is drain.go:57 (note says 62-63), rows at tui.go:1164-1199 (note says 1163-1180). The onToolResult line 75 matches.
- knock-on: onNotification and finishOldestBackground key on Background, so they skip these agents.

## doubts / not found
- not found: any live status file, any live token/progress write, any live metrics write.
- LEDGER.md lines 170-177 mention pollFiles and agentDoneMarker. Both are gone from the code (grep empty), so those lines are historical.
- no runtime run, so the background-panel diagnosis is unverified live. Session.Events body (core.go:198) not read in detail.
