# tui-view-crash: /view crash (panic in draw)

status: fixed, not committed
cause: crates/cli/src/tui/mod.rs:2406 body_shift `prev[..body_h]` panics (range end 8 > len 7) when overlay frame length != body_h
mechanism: layout() (mod.rs ~2343) used overlay_lines() result as-is; sessview list_lines() returns 1+min(n,5) rows (0 when width<10), not body_h.
  frame height drops below h; draw() second call compares prev/rows of equal short length, then slices prev[..body_h] -> panic.
  prompt_lines() already resize(hh), so only the list path broke.
repro: new test view_overlay_fills_the_body_and_redraws_at_any_size (mod.rs ~5383)
  - before fix: assert fails at 10x6 (frame rows 4 != 6); with height assert disabled, panic mod.rs:2406 "range end index 8 out of range for slice of length 7"
  - after fix: PASS
fix: mod.rs layout(): overlay Vec resized to body_h (truncate/pad with empty rows)
changed: crates/cli/src/tui/mod.rs (layout overlay normalize; new test)
checks:
  cargo test -p chaude-cli -> PASS (lib 269 passed, 1 ignored; bin 4 passed)
  cargo clippy -p chaude-cli --all-targets -> PASS (no warnings shown)
  cargo fmt -p chaude-cli --check -> PASS (exit 0)
doubts:
  - list rows beyond body_h are now cut off; when body_h < 6 the selected row may be hidden (no panic, UX only)
  - Korean long text in list/prompt paths: no panic found (wrap_cells/clip/csi_len char boundaries checked)
  - not reproduced on real TTY; repro is draw() via unit test
