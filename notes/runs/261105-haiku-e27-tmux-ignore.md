# 261105 haiku e27-tmux-ignore run
done: E27 (commit 69df85d), tmux kill (tuidiff-scroll-up-rs)
partial: ignored-test wiring 0/26
commit: 69df85d fix(engine): drained agent gets done notice only, not marked done (E27)
changed: crates/core/src/api.rs (-4 lines), crates/core/tests/engine.rs (rewrite drained test)
check E27: scripts/test-isolated.sh exit 0; clippy --workspace --all-targets exit 0 0 warn; cargo fmt --all --check exit 0
check single: engine drained_background_agent_gets_done_notice_and_stays_running PASS 4.37s
tmux: killed tuidiff-scroll-up-rs (pid 41403, target/debug/chaude); no other sessions existed
item2 triage (26 stubs, untouched): p3_ignored 12, p2:379, p2:404, final3_ignored 12
  - agent_prompt_missing (p2:379, final3:17): Go resident.go:139-141 notice+default session; Rust has no prompter slot, decision needed (spawn_tui/spawn_coder api.rs:1571,1644 fail on missing agent)
  - idle_spend (p2:404, final3:10): Go idlespend_test.go:46-82; Rust no bg spend row; feature port
  - p3 resident respawn/spawn_slot/eager/slots4/viaTUI/run_turn: Go resident_*_test seams absent in Rust; feature or seam port
  - p3 cli items (view helpers, release selection, prompt scroll, fmt_k, ctx_bar): cli lib exists now (4a9bc55) but fns private (bar.rs:90 fmt_k); tests live in core/tests (no cli dep) -> needs cli/tests move (not delete) decision
  - final3 drain tracker/idle canUse/rotate snapshot/registry wiring/stage order/input wide/panel/agent status: feature or seam gaps
doubts: item2 is not a minimal fix; 26 feature/seam ports; needs main-agent scope split
log: notes/runs/261105-haiku-parity-loop2.md appended
