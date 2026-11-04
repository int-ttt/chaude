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
