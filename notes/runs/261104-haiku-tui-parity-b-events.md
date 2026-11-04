# TUI parity part B: groups E, X, A, I26, I05 (261104)
agent: haiku-tui-parity-b
branch: rust-port (not pushed)
scope: plan/tui-parity.md E01-E28, X01-X13, A01-A04, I26; I05 recheck
go_truth: deprecated/src/main/intt/internal/harness (tui.go, repl.go, agentview.go, history.go)
rust_side: crates/cli/src/tui, crates/cli/src/main.rs (crates/core read only)

## commits
- 6444660 fix(cli): idle Ctrl+C quits like Go even with input text (E18)
  - mod.rs:704 idle Ctrl+C -> Quit; test ctrl_c_interrupts_when_busy_and_quits_when_idle_even_with_text (mod.rs:2970)
- f387d39 fix(cli): honor HARNESS_PLAIN and ensure project settings at start (X03, X04)
  - mod.rs:1984 HARNESS_PLAIN (empty = unset) gates TUI with TTY check
  - main.rs:138 settings::project::ensure_project_settings before tui::run; error printed, startup continues
- 34a32d5 fix(cli): forward SIGINT and SIGTERM to the interrupt path (E19)
  - mod.rs forward_signals (self-pipe, FD_CLOEXEC, unix) + Wake::Signal; interrupts only when app.busy
  - test sigterm_is_forwarded_as_wake_signal (mod.rs)
- 0f04a41 docs(plan): mark I26, A03, A04 audited in tui parity

## rows
- I05 done: Go tui.go:699 and Rust mod.rs Ctrl+D both quit; plan wording premise differs (delete is Delete key), no code change.
- E18 [x] fixed (above)
- E19 [x] fixed. plain line mode does not forward signals (Go forwards before the TTY check); accepted gap.
- X03 [x] fixed. X04 [x] fixed.
- A03 [x] audited. Overlays replace body rows only, chrome and input kept (mod.rs:1782-1790), matches tui.go:1349-1354. Row wording "full-screen" is inaccurate; Go is not full-screen either.
- A04 [x] audited. Esc returns to normal view: usage.rs:108, sessview.rs:337, mod.rs:629-632, mod.rs:658-665.
- I26 [x] audited. rewind.rs:36-98 matches history.go:124-152. Wiring mod.rs:638-654 and open at mod.rs:774.
- E24 [ ] differs, core-owned. agent_store.rs:780-792 has sorted_restored and dismiss_restored, but no engine restore on startup and no UiEvent carries restored agents. TUI agent_recs (mod.rs:1105) has no restored view. Not changed.
- E14 [ ] not audited this pass.
- E15 [ ] not audited. Interrupt wiring exists (mod.rs:707, 757). Panel finish is L-group.
- E23 [ ] not audited. Task-notification finish is core-dependent.

## earlier-pass results (from 261104-haiku-tui-parity-check)
- E05 [x] agentview.rs:16 MAX_AGENT_LOG=400. E22 [x] bar.rs:209-222. E25 [x] mod.rs:473-476, 1621. X10 [x] edit.rs:204-207.

## core-owned, not changed (crates/core is out of scope)
- X01 lazy engine start vs Go synchronous start-or-exit-1 (api.rs:517)
- E20 no interrupt_running(true) on shutdown (api.rs ~849-898)
- E24 restored-agent path (above)

## checks
- cargo test -p chaude-cli -> 206 passed, 0 failed, 1 ignored (after E19 test added)
- cargo clippy -p chaude-cli --all-targets -> 0 warnings
- Each code commit ran tests and clippy before committing; the docs commit has no code change.

## done / remaining
- done: I05, E18, E19, X03, X04, A03, A04, I26
- remaining: E14, E15, E23 (unchecked), E24 (core), X01/E20 (core), all other E/X/A rows not re-audited in this pass

## Result (tui-parity B remainder)
- E14: fixed. mod.rs UiEvent::Error -> dim note block + blank row (was plain "! msg"). test: error_event_is_a_dim_note_then_blank_row.
- E15: fixed. mod.rs finish_foreground() on Action::Interrupt and Answer{interrupt}; late core AgentEnd for those ids dropped (finished_fg, cleared on Busy(true)). Not called on Wake::Signal. test: interrupt_finishes_foreground_agents_only.
- E23: same for string content (bg AgentEnd removes row, done notice). diff core-owned, not edited: crates/core/src/api.rs:1948-1955 handles only Content::Text; Go onUserEvent also reads text blocks. test: background_agent_end_removes_row_and_notes_done.
- R14: fixed. rewind.rs lines() wraps rows with trunc_ellipsis; render.rs #[allow(dead_code)] removed; rewind test expects 93 x + "…".
- checks: `RUSTUP_HOME=$HOME/.rustup CARGO_HOME=$HOME/.cargo HOME=$(mktemp -d) cargo test -p chaude-cli` -> 209 passed, 0 failed, 1 ignored. clippy --all-targets 0 warnings. fmt --check exit 0.
- env: disk was full (309Mi free) during the first build; removed regenerable target/debug/incremental. Pinning RUSTUP_HOME/CARGO_HOME needed because HOME=mktemp forces a toolchain re-download.
