# TUI parity check (261104)
agent: haiku-tui-parity-check
branch: rust-port
scope: plan/tui-parity.md unchecked items vs deprecated Go TUI and crates/cli/src/tui
done_marked: 44 (I01-I25 = 25; R01-R13, R15-R20 = 19)
fixed: 1 (I16 slash completer Up/Down: browsing guard, Up at top to history, Down wraps; mod.rs:831-841, edit.rs browsing())
missing: 2 (R14 trunc_ellipsis unwired; I26 not audited)
differs: 1 (I05 premise: Ctrl+D quits in Go and Rust)
not_audited: 144 items remain [ ] (L, S, B, C, E, X, A groups and I26)
big_gap_list: notes/runs/261104-haiku-tui-parity-big-gaps.md
check: cargo test -p chaude-cli -> 205 passed, 0 failed, 1 ignored (baseline run; rerun after fix: 205 passed)
check: cargo clippy -p chaude-cli --all-targets -> 0 warnings, exit 0
not_touched: crates/core (other agent edits in progress), plan/caveman-structure.md (pre-modified, not mine)
push: none
