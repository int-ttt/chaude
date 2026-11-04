# 261104 haiku cli phase6 rows

scope: E4, E5, K5, K8, K12, S23 (plan/rust-parity-checklist.md). crates/cli/src/tui/{mod,render}.rs, plan/rust-parity-checklist.md.
not touched: crates/cli/src/main.rs, crates/core, crates/update, notes/runs other agents.

## commits (local, rust-port, no push)
- 70afe61 E4 CHAUDE_IME_SHIFT and JediTerm shift
- 716a4df E5 marked done (matches Go CLAUDE_PROJECT_DIR only)
- c8233d6 K5 clipboard off macOS (wl-copy, xclip, xsel, clip.exe)
- 396597a K12 assistant links styled after wrap; K8 marked done
- 57e6e72 S23 /update off-thread via update::text

## rows
- E4 done: ime_col_shift, App.ime_shift read once in run_tui.
- E5 done: default_src precedence matches Go; CLAUDE_PROJECT_DIR only.
- K5 done: native_copy non-macOS; not executed on a non-macOS host (only aarch64-apple-darwin toolchain).
- K8 done: Go doneLine (drain.go:40-55) = agent completion line; Rust bar::done_line at AgentEnd. Go has no per-turn done row.
  doubt: AgentEnd carries no model, so line always shows 상속 (Go shows agent model).
- K12 done: asst_text = md_bold only; body() calls wrap_linked only for rows in self.linked (assistant and user rows), wrap_cells otherwise. linkify removed (unused), its test uses restyle(mark_links).
  doubt: body() links every transcript row; Go wrapLinked only on assistant (tui.go:374) and user (tui.go:376) text.
  doubt: user text rows not checked for link styling parity.
- S23 done: /update -> Action::Update -> thread -> Wake::Note(update::text(CARGO_PKG_VERSION)).
  coordinator asked check_latest; used text() because Go /update calls update.Text -> Run (checks AND installs). check_latest alone would not match Go.

## checks (after 57e6e72 edits)
- cargo test -p chaude-cli -> 191 passed, 0 failed
- cargo clippy -p chaude-cli --no-deps -- -D warnings -> exit 0
- cargo build --workspace -> exit 0
- cargo fmt -p chaude-cli (ran; formatting applied)
- K12 test assistant_link_keeps_style_on_wrapped_rows (width 20, URL split across 3 rows, all rows carry LINK_URL_ON)
- K12 scope test only_assistant_and_user_rows_get_link_style (note row with path has no LINK_COLOR_ON; assistant row has it)
- S23 test update_notes_and_asks_the_loop_for_the_check (note + Action::Update; thread path not unit-tested)

## risks
- S23 runs a full update (install) from TUI as Go does; no test for the thread/Wake path.
- K12 scope corrected: link styling is limited to assistant and user rows as Go does (tui.go:374, 376); tool/notice rows wrap plain.
- K5 non-macOS native_copy body compiled via cfg-flip only.

## untouched in this run (other coders)
- untracked crates/cli/src/repl_args.rs (not staged)

## K12 scope correction (coordinator)
- fix: tui/mod.rs linked set (Transcript indices of assistant and user rows) filled at the asst/user push sites via link_from; body() wrap_linked only for linked rows and the live pending_main row; other rows wrap_cells.
- check: cargo test -p chaude-cli -> 201 passed, 0 failed
- check: cargo test only_assistant_and_user_rows_get_link_style -> ok
- check: cargo clippy -p chaude-cli --no-deps -- -D warnings -A clippy::wrong_self_convention -> pass; without the -A, fails only on untracked crates/cli/src/repl_args.rs:134 (other agent's file)
- check: cargo build --workspace -> exit 0
- check: cargo fmt -p chaude-cli -> applied
