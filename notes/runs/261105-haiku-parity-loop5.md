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
