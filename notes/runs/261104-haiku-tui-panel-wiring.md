# tui-panel-wiring (261104)
agent: haiku-tui-panel-wiring
branch: rust-port (not pushed)
status: partial
commit: c00d4cf feat(tui): wire agent panel, background rows and queue texts
paths: crates/cli/src/tui/mod.rs, plan/tui-parity.md

## rows fixed (plan/tui-parity.md, L group only)
- L01 chrome order bar, queue, input box, fg panel, bg rows, status (mod.rs chrome ~1334)
- L03 starting… only for zero size (layout ~1925). Runtime reads terminal size first: unreachable, deliberate
- L05 welcome box clamp(4,64) vs Go min 20: deliberate deviation, fits narrow terminals
- L06 bg rows = background && !done (agent_lines, agent_row_line)
- L07 fg rows = !background && !done, "… 1-n/N (마우스 휠로 스크롤)" overflow row
- L08 MIN_VIEW_ROWS=3, MAX_QUEUE_ROWS=5, MAX_PANEL_ROWS=4
- L11 queue text from UiEvent::QueueTexts (core 8f1dba7), was blocked-core
- L14 cursor() subtracts fg+bg+2 below-box rows

## checks
- cargo test -p chaude-cli tui: 201 passed, 0 failed, 1 ignored (after cargo fmt)
- cargo test --workspace x3 (HOME=mktemp, CARGO_HOME/RUSTUP_HOME real): exit 0 each, passed 621, failed 0, ignored 66
- cargo test -p chaude-core --test engine x3: 68 passed, 0 failed each (~65 s)
- cargo clippy --workspace --all-targets: exit 0, 0 warnings
- cargo fmt --all --check: exit 0 (fixed 4 mod.rs diffs in my code via cargo fmt -p chaude-cli)
- parity_final3.rs fmt: no diff now (not touched by me)

## flaky engine.rs timing test
- status: NOT reproduced in 6 runs (3 engine-only, 3 workspace). No code changed. No assertion touched.
- candidate (unconfirmed): crates/core/tests/engine.rs idle_extra_coder_expires_and_base_coder_stays (~line 1816)
  uses `kill -0 <pid>` for "extra coder expired". A killed child that is not yet reaped still answers kill -0,
  so the 5 s poll can fail under load. Fix would reap/wait the child, not widen the bound. Not applied (no repro).
- older notes name load-sensitive engine tests (budget_interrupts_turn_once, background_only_agent_does_not_queue,
  background_agent_replay_has_no_agent_end, handoff_snapshot_lists_running_background_agent, slow_agent_result...).

## doubts / gaps
- index was shared with concurrent S/B/C audit agent (staged its own hunks). Committed only my two files; its
  S11/wheel hunks landed in 08ba69f, B rows in dad7627 before my commit. No reset/stash.
- panel mouse-wheel scroll (Go panOff) not wired. Go popH (slash popup height) not subtracted from queue/panel room.
- AgentRow has no Summary/Log/Failed/Interrupted; panel state always 실행 중 (only running rows listed).
- L03/L05 are documented deviations, not code-matched to Go.
