# 261105 haiku parity loop5

queue:
- 1: drain log sink (crates/core/src/api.rs drain lines, metrics -> transcript, Go repl.go:994-998 + switchchild.go:103-106)

## round 1

- item 1 drain log sink: DONE, commit fced34b
  - go: repl.go:997 `r.tr.add(kind, kv{"type", ...}, kv{"session", ...})`; switchchild.go:106 same shape; Transcript.add (repl.go:105-117) writes {"t", "kind", ...fields}
  - rust: drain lines now go to cfg.transcript via State::log_transcript; metrics write removed (Go has none)
  - test: engine.rs drained_child_prompt_is_denied_and_lines_are_logged asserts transcript rows (kind/type/session/t); metric_rows helper removed
  - checks: test-isolated EXIT=0, 707 passed / 0 failed / 3 ignored; clippy no warnings; fmt --check clean
- loop4 log committed alone: a37b13d
- note: queue holds only item 1 (no other queue list found in notes/runs)

## round 2

- item 2 idle tick: DONE, commit 9671d2d
  - go: tui.go:454 `if n := m.runningN(); m.busy || n != m.lastRun { m.refresh() }` (spinner.TickMsg); lastRun set at tui.go:403 in refresh; runningN tui.go:1445
  - rust: crates/cli/src/tui/mod.rs tick() gate (~665-673): re-read agent_src only when busy || n != last_run; last_run field (~131), AgentRows arm sets last_run (~607); running_n no longer cfg(test)
  - test: crates/cli/src/tui/mod.rs `tui::tests::idle_tick_gate_skips_unless_busy_or_count_moved` (count 0->1 fires; idle same-count skips; busy fires; idle same-count skips; 1->0 fires); existing idle_tick_refreshes_when_background_finishes still passes
  - checks: test-isolated EXIT=0, 708 passed / 0 failed / 3 ignored; clippy EXIT=0 0 warnings; fmt --check EXIT=0
  - size: du -sh 4.0G
- open: round-2 log entry left uncommitted (only item commit requested)

## round 3
- item 3 (loop4 R5 item2 non-coder role case): DIFF, fixed, commit 6ab5e3a fix(core)
  - go: livechild.go:42-49 parkedIdleEvent answers every parked child's can_use_tool (`case t == "control_request" && ... "can_use_tool"`, deny at :48 `"denied: session parked (another model is active)"`); resident.go:155 wires it for every resident slot (tui, prompter, coder); resident_coder.go:85-88 coderMayUse false for non-coder role (`!strings.HasPrefix(c.Role, "coder")`).
  - rust before: on_prompter_line (api.rs:1578) and on_tui_line dropped non-result lines, so tui/prompter can_use_tool got no reply (child hangs). Coder path (on_coder_line, coder_permission) was already right.
  - rust after: api.rs:1578 on_prompter_line and api.rs:2022 on_tui_line check can_use_tool first and call deny_parked (api.rs:4052, helper; write error ignored like Go `_ =`).
  - checks: scripts/test-isolated.sh rc=0, 708 passed, 0 failed, 3 ignored (28 result blocks); cargo clippy --workspace --all-targets rc=0, 0 warnings; cargo fmt --all --check rc=0.
  - DOUBT: no new test covers the tui/prompter deny path (needs a fake child emitting can_use_tool on those roles); verified by code read + green suite only.
  - DOUBT: loop4 R5 item2 doubt "non-coder role not modeled" is now closed by code, not by a test.
- loop5 round 2 log committed alone: 96d1769 (docs(notes)); this round-3 entry left uncommitted.
- disk: repo du -sh 4.0G before fix (under 15GB).
- NOT DONE: none for items (a) and item 3. Not pushed. No reset/stash/tag/amend.

## round 4
- item 4 (agent_tracker_drain_half proxy, api.rs ~4356): choice = real assertion (test-only), commit 8eacfc3 test(core)
  - go: drain.go:305-313 drain.finished reads only d.tr.running() + since/timeout (no file, no quiet period); drain_test.go:11-59 TestAgentTrackerAndDrain.
  - rust: api.rs:4356-4420 go_finished (test-local) = drain_finished(tr.running().len(), elapsed, timeout) mapped to Go (done, timedOut); old proxy comment was wrong (said Go reads file/quiet period).
  - checks: cargo test --lib agent_tracker_drain_half 1 passed; scripts/test-isolated.sh EXIT=0, 708 passed / 0 failed / 3 ignored; clippy --workspace --all-targets 0 warnings; fmt --check rc=0; du -sh 4.0G.
  - DOUBT: production seam: reap_drains (api.rs:3552) counts self.agents (HashMap<String,Instant>, api.rs:619), not AgentTracker; Go drain holds tr. Not fixable test-only; left as is.
  - DOUBT: design D5 said #[ignore] if drain test-only; drain struct is production in Go (switchchild.go:101), so no ignore (brief forbids #[ignore]).
- loop5 round 3 log committed alone: b2d5bd6 (docs(notes)).
- NOT DONE: no push, no ignore, no test for on_prompter_line/on_tui_line deny (item 3 doubt stands).
