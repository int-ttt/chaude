# e2e hang test, chaude 0.6.5 (installed ~/.local/bin/chaude), 261029
result: hang FIXED (TUI + plain). NEW finding: late background-completion reply is not shown in UI.
driver: notes/runs/261029-e2e-hang-drive.py (python pty; TUI = pty default, plain = HARNESS_PLAIN=1; needs OSC11 reply + text, 1.5s, then Enter)
prompt: main(haiku) launches bg Agent(haiku) running `echo ok > done.txt`, main replies "launched" and ends.

## TUI (pty)
- 4.5s prompt sent; main "launched" at +7.3s (transcript 120.2)
- transcript 'permission_idle' Bash at +0.4s after the assistant reply (== permission request arrived AFTER main turn ended; idle reader handled it)
- TUI showed "Allow Bash(echo ok > done.txt)? [y] yes [n] no"; answered y at ~12s -> "allowed"; done.txt=ok at ~13.5s (run 3 of 3 identical: 1.5s after prompt)
- PASS: done.txt exists, no hang (3 runs, same result). Old bug would have hung ~6.6min and AbortError.
- FAIL/gap: UI status stayed "agents 1 running (bg 1)" for 60s after done; no completion notice rendered.
  Evidence: child session 3b89f0b6 jsonl: task-notification enqueued 16:33:24.5Z, child replied 16:33:27.5Z ("백그라운드 에이전트가 완료되어 `done`을 보고했습니다 ..."); chaude transcript.jsonl has NO assistant record after "launched" and the pty log shows no such text.
  inference: idle reader consumes child-initiated turn events (noteIdle) but does not render assistant text/tracker "agent done" -> user never sees the notice; status counter stale.

## plain (HARNESS_PLAIN=1)
- no hang; printed: `[permission denied while idle] Bash {"command":"echo ok > done.txt",...} (use /mode auto or acceptEdits)`; transcript permission_idle Bash. done.txt not created (expected). Prompt returned immediately (">").
- same gap: no subagent follow-up text shown within 200s.

## cleanup
tests/e2e-hang removed, .gitignore entry reverted. ~/.claude/projects/*e2e-hang-* session dirs left (outside repo).
cost: ~5 haiku runs, <$0.03

## fix re-run (local build, 0.6.5+idle-render)
fix: idle reader renders child-initiated turns (agent-done follow-up), writes transcript source=background + metrics stage=turn source=background, clears bg agent counter (task-notification parse, fallback at result)
late-result guard: kept (inactive/empty result ignored); unit TestIdleBackgroundFollowupRendered incl. bare late result
tui: PASS - transcript "assistant background 백그라운드 에이전트가 완료되어 `done` ..." ; status line shows no "agents" segment (counter 0)
plain: PASS - reply rendered+logged after idle deny
note: earlier TUI "stall" was the driver exiting before the reply finished (POST wait too short), not a chaude block
checks: gofmt/vet/build/go test -race pass
