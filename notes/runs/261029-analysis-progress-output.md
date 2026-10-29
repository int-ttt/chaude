# analysis: live progress output for Haiku/subagent work

date: 2026-10-29
source: only notes/runs/261029-scout-progress-signals.md (no repo files read). Overview-cap facts = context only.
legend: [F] fact from scout (code reading, no runtime run) / [I] inference / [O] opinion

## (1) feasibility without new I/O
- [F] sub-agent results arrive at completion only (drain.go:72-79); final result is first line only in UI.
- [F] sub-agent text/tool lines stream into agentRec.Log in memory (agentview.go:19-66, cap 400). No file written by onSubEvent.
- [F] in-memory reads are mutex lock + copy, no I/O; 8 Hz spinner tick (125 ms) already calls running() (tui.go:327, 372-378).
- [F] panel rows and status line "agents N running" already read tracker state (tui.go:957-978, 1164-1199).
- [F] no status file, no live token value; onSubEvent ignores sub-agent usage (agentview.go:26-41).
- [F] agents.json poll = ~21 KB/read, changes only on state transitions; /usage poll spawns `claude -p`. Both unsuitable.

verdict: [I] feasible with zero new I/O, as an in-memory display change.
can show (all derivable from existing tracker state):
- running / done / failed / interrupted per agent [F: status exists]
- elapsed time (now - started; started field exists, agentstore.go:28-43) [I]
- last tool use: last "● tool(args)" line of Log [I]. Needs a small accessor returning only the last line; do NOT call snapshot() per tick (copies up to 400 lines) [F].
- count of agents running (already shown)
cannot show:
- sub-agent token progress: usage field is dropped at onSubEvent, never stored. Would need new capture code in onSubEvent (in-memory, still no I/O) [I]. Even then, per-event usage only reflects what the stream emits; unverified [uncertain].
- percent complete / ETA: no signal exists [I].
caveat [F]: background-panel bug (real Agent tool_use lacks run_in_background; "Async agent launched" marks Done) is unfixed, so background agents currently vanish from the panel. Progress display would show nothing for them until that is fixed. Not runtime-verified.

## (2) fit with design
- [I] Haiku main + subagents doing work: the visible gap is that the user sees nothing while a subagent runs for minutes. Elapsed + last tool line fills that gap.
- [I] long-context constraint: display reads happen in the TUI process memory, not model context. No file read added to main, no tokens added to the model prompt. Compatible. Risk only if someone proposes feeding progress to main model (would grow context) - do not.
- [I] cost/complexity: small (accessor + render in existing panel row). Touches tui.go and agentview.go, which are already modified in the working tree (git status) - merge/confusion risk with uncommitted work [F: both files listed modified].
- [I] dependency: value is low until the background-panel fix lands, since background agents are the long-running ones.

## (3) recommendation
- [O] Defer, then do a minimal version: elapsed + last tool line, in-memory only, after the background-panel fix is applied and committed. Drop token progress (not available; new capture buys little).
- Reason: feasible and cheap, but depends on an unapplied fix and on files with uncommitted edits; doing it now stacks changes on unverified diagnosis.
- Not "drop": it is the only cheap answer to "is it still working?".

## single user decision
- [O] Should the background-panel fix be applied first, with the progress display (elapsed + last tool line, no tokens) as a follow-up step right after? (yes = defer-then-do as above; no = drop progress output)

## open / uncertain
- no runtime run; Session.Events body not read.
- whether sub-agent stream events carry usable usage: unverified.
