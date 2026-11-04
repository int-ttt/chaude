# tui-parity A final (261104)
agent: haiku-coder
branch: rust-port (not pushed)
scope: plan/tui-parity.md rows C10 C11 C12 C13 C16 C17 C19-C25 C27 C28 C30 C32, B01 B07 B08 B09 B18 B19 B20 B21-B24
disk: start 325Mi free; cargo clean removed 219619 files (12.7GiB); after 6.5Gi free
status: partial (batch 1 of ~3)

## batch 1: C13 C16 (done)
- fixed C13: sessview.rs SessView::key(key, width, vp_h) level 2 now matches Go svKey (sessview.go:162-204): up/k, down/j, pgup, pgdn, home, end scroll the body; left/right step prompts and reset off; esc resets off.
- fixed C13 wheel: SessView::wheel(up, width, vp_h) level 2 scrolls off±3 (Go tui.go:545-554); level 1 unchanged (wheel_sel).
- fixed C16: prompt_lines header "<short id>  prompt n/N" plus dim "lines a-b/len" counter and body window [off, off+hh-1); "… (+N lines)" removed (Go svPromptLines sessview.go:233-246).
- clamp: scroll() clamps to len - max(1, vp_h-1) (Go svScroll sessview.go:156-158).
- call sites: mod.rs App::on_key sess dispatch and overlay_wheel pass (text_width(cols), body_h(cols, rows)), the same values the frame uses.
- new test: sessview.rs prompt_scroll_is_clamped_and_resets_on_switch.
- diff C13: level 1 row shows short id, not Go agentLabel (sessview.go:32-40); needs role data not in own_sessions.
- diff C16: header uses short id, not agent label; same root cause.
- sed artifact: a stray crates/cli/src/tui/sessview.rs-E (BSD sed -i -E backup) was created and removed.

## checks
- HOME=$(mktemp -d) (CARGO_HOME/RUSTUP_HOME real) cargo test -p chaude-cli sessview -> 10 passed, 0 failed, 0 ignored
- HOME=$(mktemp -d) cargo test -p chaude-cli tui -> 206 passed, 0 failed, 1 ignored
- cargo clippy -p chaude-cli --all-targets -> 0 warning/error lines
- cargo fmt --all -- --check -> exit 0
- workspace test not run in this batch (acceptance pending)

## audited so far
- C13 fixed, C16 fixed (this batch)
- earlier (run 261104-haiku-tui-parity-a.md): C12 same; C18 C29 C31 same; C33 fixed.

## remaining
- audit: C10 C11 C17 C19-C25 C27 C28 C30 C32; B01 B07 B08 B09 B18 B19 B20 B21 B22 B23 B24
- B21-B23: core already emits QueueTexts and merges with "\n\n" (api.rs submit ~985, next_turn ~2638); TUI consumes it. Verify, not blocked. B23 needs check that busy excludes background agents.
- C22: no /clear handoff snapshot in core (api.rs send_turn ~1745); core-side, unverified.
- C17: /import is local in Rust (import_view), Go sends an engine turn (tui.go:1051-1057); unverified diff.
- acceptance: workspace test x2, clippy --workspace --all-targets, fmt --all --check, git status clean of my changes.
