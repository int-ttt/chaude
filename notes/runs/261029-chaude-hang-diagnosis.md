# chaude hang diagnosis (subagent Bash 01:09:04 KST = 16:09:04Z 10-28)
cause: no idle-time stdout reader. control_request(can_use_tool) from a still-running background subagent arrived after the main turn's `result`; nobody read/answered it.
confidence: high on mechanism (code + transcript agree); medium on "turn had already ended" (inferred from timing, not a logged result event)

## evidence
- subagent transcript agent-a92b1bedcae8b69d0.jsonl line 102 (16:09:04.195Z) Bash tool_use `gofmt -w ... && go build ./... && go test ./...`; line 103 (16:15:43.635Z) tool_result is_error: "Tool permission request failed: AbortError: Tool permission stream closed before response received". Next Bash (16:15:47) likewise "Stream closed". => no bash ever started; the call was blocked at the permission step (waiting for the --permission-prompt-tool stdio answer) ~6.6 min until chaude quit closed stdin. Not a gofmt/self-modify issue.
- ~/.claude-harness/transcript.jsonl: last user 16:08:51Z ("chaude 맞아"), last "permission" record 16:08:55Z (main Bash, allowed), then assistant reply. NO permission record for 16:09:04 => askPermission never ran for it.
- core.go Session.Events(): reads s.lines only inside runTurn; returns on first `result` (core.go:197). repl.go handles control_request only in runTurn's handle() (repl.go:277). Between turns (REPL idle at prompt) nothing consumes s.lines (chan buffer 256, so child stdout not blocked, just unread).
- drain.go consume() auto-denies, but only for rotated "draining" old sessions; the live r.sess has no equivalent idle consumer. Not involved here (no rotation logged).
- Background subagent outlives the main turn (agents run_in_background; "백그라운드 완료 감지 불동작" noted by the agent itself), so its tool permission requests are forwarded to the parent's stdio control channel after `result`.

## ruled out
- drain auto-deny: would have produced a deny message, not AbortError; no drain event.
- stdout pipe blocking: buffered goroutine reader, and child was [waiting] not blocked on write.
- permission UI (ui.ask) blocking: no permission record at all for that request.

## proposed fix
1. Start an always-on reader goroutine per live Session (or make Events-loop owner a single dispatcher): when no runTurn active, consume s.lines; on control_request can_use_tool route to askPermission (same mode/ui path, ui.ask works from idle if it posts to TUI), on other events (assistant/user/stream_event from subagents, task notifications) feed tracker + optionally UI; on `result` mark idle. Guard with a mutex/"turnActive" flag so runTurn and idle reader never both read (e.g. runTurn takes ownership by pausing the idle reader, or one dispatcher forwards to the active turn handler via channel).
2. Minimal variant: in the REPL input loop, poll s.lines with a ticker (select on input and s.lines) and handle only control_request + result while idle.
3. Fallback safety: if no UI can answer within N s, respond deny with message instead of leaving child waiting; log kind "permission_idle" to transcript.
4. Test: fake session emitting result, then control_request; assert RespondPermission called without a new runTurn.
open: does a post-result event also include late `result` for the background task (could be mistaken as a new turn's result)? check when implementing.

## fix implemented (261029)
- core.go: Session.StartIdle/StopIdle; idle goroutine reads s.lines when no turn; Send pauses it (held token), Events resumes it at end (no race where idle eats a turn's result).
- repl.go: start() starts idle reader -> Repl.idleEvent: can_use_tool -> askPerm(sess, ev, idle=true) (mode/TUI ui.ask as in-turn); plain path idle with no auto-allow: denied with message + note + transcript "permission_idle" (stdin belongs to prompt loop; cannot prompt). rate/tracker events fed. rotate: StopIdle before drain; Close stops it.
- stray result: noteIdle marks child-initiated main turn (assistant/stream_event w/o parent); Send during it => skip++ so Events discards the earlier background `result`. Inference: no real log of the late result was found; heuristic.
- tests (idle_test.go): control_request after result answered (auto mode), denied on plain no-prompt, stray late result skipped. go test -race ok.
- not done: N-second timeout fallback for TUI ask (user must answer prompt).
