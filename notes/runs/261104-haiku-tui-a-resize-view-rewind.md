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
