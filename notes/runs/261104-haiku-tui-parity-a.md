# TUI parity part A (261104)
agent: haiku-tui-parity-a
branch: rust-port
scope: plan/tui-parity.md groups L (L01-L14), S (S01-S10), B (B01-B25), C (C01-C33)
owner-excluded: E, X, A, I26 (other coder, concurrent)
status: partial
audited: 0 of 82 (no [x] marks written in this run)
fixed: 0
commits: none
check: not run (no code change made)
push: none
core: not touched

## findings so far (read-only)
- L06/L07/L08/L11: Rust has no panel, bg, or queue rows. chrome() mod.rs:1282 draws only top bar, input box, status row. Go panelLines/bgLines/queueLines (tui.go:929-946, 1458-1505) unported.
- L11 queue text: core emits Queued(usize) only (crates/core/src/api.rs:960, 2544). Text rows blocked without core change.
- R14: render.rs:316-331 trunc_ellipsis has no production caller. Go ansiTrunc (tui.go:1437) is used for overlay rows (tui.go:1299, 1317, 1339) and agent rows (tui.go:1494). Rust rewind rows (rewind.rs:99-119) use bar::clip (no ellipsis). rewind.rs is unwired into mod.rs and I26 belongs to the other coder, so wiring there collides with their rows.
- Top bar: bar.rs:164 clips with bar::clip (no ellipsis). Go header is not ansiTrunc'd, so no change needed there.
- Go statusLine text (tui.go:1201-1228) vs Rust status row: not yet compared line by line.

## done
- read: plan/tui-parity.md group sections, Go tui.go key ranges, bar.rs, render.rs, rewind.rs, overlay.rs, mod.rs App/chrome/layout.

## remaining
- audit L01-L14, S01-S10, B01-B25, C01-C33 with file:line evidence.
- TUI-only fixes: panel rows (needs agent model/name; degraded from desc), bg rows, status line parity, R14 wiring (after coordinating with I26 owner on rewind.rs).
- blocked: L11, B21-B23 (core queue text), R16/R17/R07 (core).
- per-group commits via git add -p; cargo test -p chaude-cli; cargo clippy -p chaude-cli --all-targets.

## batch 1: group L (done)
- audited: 14 of 14 (L01-L14), marks written in plan/tui-parity.md
- same: L02, L04, L09, L10, L12, L13 (6)
- diff: L01, L03, L05, L06, L07, L08, L14 (7)
- blocked-core: L11 (1)
- fixed: none (no code change; panel/bg/queue rows deferred, see L06/L07)
- L06/L07 note: AgentStart/AgentEnd/AgentBackground exist in core, so CLI wiring is possible; not done in this batch because the panel coder is changing the same events now.
- L03 note: Rust reads the terminal size at startup, so the "starting…" frame never shows; left as diff.
- check: not run (docs-only change)
- remaining: S01-S10, B01-B25, C01-C33

## batch 2: group S (done)
- audited: 17 of 17 (S01-S17 in plan/tui-parity.md; the brief said 10 but the plan has 17 rows), marks written
- same: S01, S02, S03, S04, S05, S06, S07, S08, S09, S10, S13, S14, S15 (13)
- fixed: S11 (copy_text: native copy first, then OSC 52 even on native error; mod.rs copy_text, Go selection.go:115-119), S12 (wheel ±3 for agent view; PgUp/PgDn still diff, see plan row)
- diff: S12 (PgUp/PgDn page = CHROME-based height, not live vp.Height-2), S16 (panelWheel not ported, blocked by L06), S17 (scroll_range_text bar.rs:354 unwired)
- code fixes: settings.rs Picker::wheel (Go settingsRows=3); agentview.rs AgentView::wheel (+3 up, -3 down floored at tail, Go tui.go:560-567) with test wheel_scrolls_the_log_by_three_like_go; mod.rs overlay_wheel routes settings then sess, recs, pick, then av (Go tui.go MouseMsg order 540-567)
- test: wheel filter 4 passed (agentview, usage, sessview, records)
- check: HOME=$(mktemp -d) cargo test -p chaude-cli -> 220 passed, 0 failed, 1 ignored (includes the other coder's uncommitted panel tests in the working tree)
- check: cargo clippy -p chaude-cli --all-targets -> 0 warning/error lines
- check: cargo fmt -p chaude-cli -- --check -> FAIL, but only in the other coder's uncommitted panel code (mod.rs:1388 agent_lines, 1900 agent_row_line, 2828/2851/2860 panel tests). None of my hunks are in the reported ranges. Not touched.
- staging: mod.rs staged from HEAD + my hunks only (overlay_wheel, scroll comment restore, copy_text) via a blob built from HEAD; working tree keeps the other coder's panel edits. settings.rs and agentview.rs staged whole (only my changes).
- commit: see git log for the S group commit (message: fix(tui): S11 copy order, agent view and settings wheel)
- remaining: B01-B25, C01-C33
- blocked-core: B21-B23 (queue text, core emits Queued(usize) only)
