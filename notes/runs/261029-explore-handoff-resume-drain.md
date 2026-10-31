# explore: handoff.go / resume.go / drain.go (read-only)
role: pure helpers for context rotation (handoff note validation, reco carry-over), /resume import of Claude Code transcripts, and old-child drain tracking. No edits.

## handoff.go (88 lines)
- extractReco(reply): last line starting with "추천:" (leading * # > space ignored) to end of reply; "" if none.
- validateHandoff(note, reco, changed): problems list; checks rune len >= 200, sections (case-insensitive substring): goal | decision | files | in-progress/"in progress" | open question | preference/instruction | exact; each changed file basename must appear in note.
- withReco(reco, snapshot): prepends "## Previous recommendation (verbatim)" + reco.
- recoPreserved(reco, firstMsg): byte-exact substring check.

## resume.go (122 lines)
- claudeProjectDir(cwd): ~/.claude/projects/<cwd with / . _ replaced by ->.
- listClaudeSessions(dir): *.jsonl, skip unreadable/0-turn, sort by mtime desc.
- sessionList -> resumeSourceMsg: only "" or "claude" accepted.
- importSession(arg): resolveClaudeTranscript -> readClaudeTranscript -> r.clear() -> r.applyImport(imp, path) (importclaude.go:151: snap = withReco(imp.Reco, buildSnapshot(note)); preserved = recoPreserved) -> printed as "import: ... reco_preserved=..." (sent with next user message).
- resumeLine: plain loop, lists up to 20, reads number, importSession(id).

## drain.go (215 lines)
- agentTracker: map of agentRec keyed by tool_use id (name Agent/Task); onToolUse, onToolResultText ("Async agent launched" => Background), onNotification (<task-notification> + <tool-use-id>), finishForeground, finishOldestBackground, running, counts, sortedRunning; persists via store.
- drain struct {sess, tr, since, timeout}: finished(now) = no running agents (done) or elapsed >= timeout (timedOut).
- consume(log): reads old child stdout until close; auto-denies can_use_tool control_request ("denied: session is draining"); logs drain_deny / drain_event.

## mechanism (repl.go call sites)
- makeSnapshotN (repl.go:675): up to 2 handoffPrompt turns; validateHandoff unless NoValidate; invalid -> note replaced with failure text, state snapInvalid; no note and no turns -> snapHeld.
- rotate (repl.go:703): if pendingSnapshot set -> restart child, keep snapshot. Else snapshot; if agentsBusy && MaxDrain>0: StopIdle, append drain (since=now, timeout=DrainTimeout), evict oldest beyond MaxDrain (Close), go consume; drained=true. Else Close. Then pendingSnapshot=snap, start new child, log rotate (outcome drained|closed).
- pollDrains (repl.go:782): finished drains removed; notice queued for next user msg.
- closeDrains (repl.go:773): at exit, close all draining sessions, log drain timed_out=true at_exit=true.
- extractReco used in importclaude.go:96 and repl.go:846 (reply capture).
- finishOldestBackground: repl.go:362. finishForeground: tui.go:436,472.

## not found / doubts
- resolveClaudeTranscript and readClaudeTranscript are defined outside the three files (not read).
- drain timeout default (r.a.DrainTimeout) and MaxDrain default not checked.
- handoffPrompt text not read.
