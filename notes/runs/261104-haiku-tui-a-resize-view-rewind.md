# tui-a run: resize / view / rewind (2026-11-04)
status: partial
done:
- rewind tui-diff scenario added and committed (7977271). scripts/tui-diff.sh ALL_SCENARIOS now 15 entries (rewind last).
- core needs written: notes/runs/tui-overlays.md (prompter/tui Stage events; TurnStats ctx and result_* fields).
- baseline worktree /tmp/tui-base removed.
numbers (tmux 100x30, run now):
- rewind: 10% (30/30 rows). Picker cut and rewound note work; mismatch is core-side stage rows and ctx line shift.
- view: 90% (27/30). Mismatch rows 10-11: no-sessions note wraps at 99 cells in Rust, Go wraps at 100 (keeps trailing "-"). Note has no width flag (note() stores plain strings).
not done:
- resize-80x24: 12% now (baseline 7699492 = 20%). Cause: Stage rows shift bottom rows; prompter row missing (core). Not fixed TUI-side; masking duration rejected.
- submit-stream / tool-use / scroll-up: 13% each, row shift from core-side gaps (see tui-overlays.md).
- render/bar R03..B17 hooks and allow(dead_code): not done. bar.rs, render.rs are TUI-B, uncommitted; commands.rs:164 allow(dead_code) remains.
risks:
- view 95+ needs note-specific wrap width (transcript flag) in TUI-B render.rs wrap path.
- rewind has no unit test.
update (2nd turn):
- view: 93% (28/30). /view error note now wraps at width+1 (wide transcript indices) and starts grey. Remaining rows 11-12: Go reset-only row 12 (ESC[39m) vs Rust reset folded into row 11. Tried FG_OFF row and FG_OFF+space row; both gave the same 93%. Not solved.
- rewind unit test: mod.rs double_esc_rewinds_to_selected_prompt (PASS).
- render/bar R03..B17 hooks: NOT wired. Still "yes" in notes/runs/tui-render-bar-edit.md: R03 R04 R08 R09 R10 R11 R15 R19 R20 I30 I32 I33 B04 B05 B06 B07 B10 B15 B17. allow(dead_code) kept in bar.rs, render.rs; commands.rs:164 kept.
- bar.rs/render.rs WIP comments kept and committed (document unwired items).
- acceptance: cargo build -p chaude-cli PASS; cargo test -p chaude-cli 159 passed; cargo clippy -p chaude-cli -- -D warnings PASS.
