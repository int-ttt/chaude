# 261104-haiku-tui-render-decouple
scope: crates/cli/src/tui/mod.rs only. crates/core not touched (mid-edit by other coder at time of run).
hot_spots:
- frame(): App::body() rewrapped the whole transcript (wrap_cells + wrap_linked) on every draw. 2200 rows -> ~11-13 ms/frame release.
- tui loop: one draw per wake batch, no frame cap. 200 events/s -> up to 200 full rewraps/s.
- engine handle calls (submit/interrupt/usage/...) are channel sends already; engine thread already off UI. no change needed.
- copy_text() runs pbcopy synchronously on mouse release (one-off, not per frame). not changed.
- editor.take() writes history file synchronously on submit (one-off per submit). not changed.
changes:
- WrapCache (RefCell in App): per transcript line keep (hash(text,width,linked), wrapped rows). body() rewraps only changed lines; frame reads cache.
- wrap_key / wrap_row helpers (same wrap logic as before).
- tui loop: dirty flag; frame drawn at most every FRAME=16ms (about 60 fps); wait_batch timeout = time to next due frame while dirty; TICK still drives spinner when animating. Keys/output in a batch applied before the draw check.
- bench: tui::tests::bench_frame_large_history_streaming (#[ignore]); 2000-row history, 200 streamed text events, one frame each.
frame_ms (release, bench, avg of 200 frames):
- before: 11.376 / 13.300 (p95 11.68 / 22.0) [two runs]
- after: 0.275, 0.284, 0.310, 0.669(noisy run), 0.459 (final tree) [p95 0.29-1.5]
- loop rate under 200 events/s: <=62.5 frames/s by construction. not measured end to end.
check:
- cargo test -p chaude-cli (in clean HEAD snapshot + this mod.rs, because working-tree core did not compile mid-edit) -> 204 passed, 0 failed, 1 ignored
- cargo clippy -p chaude-cli --all-targets -> 0 warnings
- cargo fmt -p chaude-cli -- --check -> clean
scroll: no change to scroll state logic; existing scroll/typing/new-output tests pass.
doubts:
- working-tree crates/core failed to compile during this run (other coder). verified via snapshot of HEAD core.
- end-to-end loop rate and key latency (<=16ms wait while dirty) not measured in a real TTY.

## follow-up: changed parts only (user correction)
user: "아니 한번에 그리지 말고, 바뀌는 부분만 바꿔" (repaint only what changed).
still_full_redraw_before:
- body shift on streamed line: follow-tail moves every body row up by 1, so all 36 body rows were repainted (ESC[2K + row text).
- draw() itself already diffed rows; no terminal.clear() on normal updates. Resize still does Clear(All) (expected).
changes (crates/cli/src/tui/mod.rs):
- layout(app,w,h) -> (rows, body_h); frame() is now #[cfg(test)] wrapper.
- body_shift(prev, rows, body_h): detects body moved up/down by 1..8 rows (k=0 first).
- draw(): when shifted, DECSTBM to body rows (ESC[1;{body_h}r), ScrollUp/ScrollDown(k), reset ESC[r; model prev rows as moved (scrolled-in rows blank), then diff as before. Chrome/input/status rows still diffed normally.
- test draw_scroll_region_leaves_the_screen_equal_to_the_frame: small VT model (CSI H/K2/S/T/r/m/h/l) fed draw() output, asserts screen == frame after each update incl. scroll -3,-2,5,100,-40, tail append, 20-line burst (k>8 full path). Also asserts a streamed line emits ESC[1S.
- bench: per update counts bytes and repainted rows (ESC[2K count in output) and cells = repainted rows * cols.
measure (release bench, history 2000 + 200 streamed = 2200 rows, 120x40, one streamed line per update, n=200):
- before (bytes avg/max): 3080 / 5985 (measured as logical diff; old draw repainted the same 36 rows)
- before (rows repainted avg/max): 36 / 36
- before (cells repainted avg/max): 4320 / 4320 (36 rows * 120 cols)
- after (bytes avg/max): 104 / 128
- after (rows repainted avg/max): 1 / 1
- after (cells repainted avg/max): 120 / 120
- frame ms after: avg 0.278 p50 0.278 p95 0.288 (unchanged from prior run, draw cost is not the bottleneck now).
check:
- cargo fmt -p chaude-cli (applied to mod.rs only)
- cargo test -p chaude-cli -> 205 passed, 0 failed, 1 ignored
- cargo clippy -p chaude-cli --all-targets -> 0 warnings
- cargo test -p chaude-cli --release bench_frame -- --ignored --nocapture -> BENCH lines above
still_full_redraw_after:
- scroll moves larger than 8 rows, resize (Clear All), and overlays that change content (usage/sessions list) repaint changed rows only, but there is no scroll shortcut beyond 8 rows.
- not measured: busy spinner ticks (expected 1 status row per tick), real TTY/JediTerm support for DECSTBM + SU/SD (VT100 standard, not verified in a real terminal).
core change needed: none. crates/core not touched.
