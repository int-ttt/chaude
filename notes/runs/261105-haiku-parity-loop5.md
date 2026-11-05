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
