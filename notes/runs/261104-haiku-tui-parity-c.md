# tui-parity C audit (261104)
agent: haiku-coder
branch: rust-port (not pushed)
scope: plan/tui-parity.md C rows C10 C11 C12 C17 C19 C20 C21 C22 C23 C24 C25 C27 C28 C30 C32
disk: 629Mi free at end (shared target 1.3G); a second full build was refused (ENOSPC), so staged-tree compile was not run
status: done (audit 15 rows; fix 1)

## verdicts
- same (10): C11 C12 C17 C20 C21 C24 C25 C27 C28 C32
- fixed (1): C19
- diff (4): C10 C22 C23 C30

## fixed
- C19 /version: version.go:23-36 port now commands.rs claude_version(): resolve_bin("claude") --version, 3s cap via try_wait poll, "unknown" when missing/failing/timing out, cached in OnceLock.
- /version no longer reaches the engine: mod.rs submit guard `!matches!(first, "/status" | "/mode")` (was: /status /version /mode). Before, the raw "/version" went to the Claude child and the note was never shown.
- Ctx.claude_version field removed (was always ""); dispatch "/version" calls claude_version() only when /version is typed.
- test: commands.rs version_note_shows_chaude_version (asserts version_note format, local Note prefix, non-empty claude_version).

## same (evidence)
- C11 RecPicker keys/wheel records.rs:330-352 (Go keys), render records.rs:354-383.
- C12 open sessview.rs:256-278, match_sessions sessview.rs:193.
- C17 import_view/show_import mod.rs ~1010-1040; mechanism differs from sendImport tui.go:1043, result matches.
- C20 Update off-thread mod.rs:2466-2474 via Wake::Note; UPDATE_START_NOTE commands.rs:192.
- C21 usage.rs:104-108 Esc/Enter close; test usage.rs:217-221; one-shot api.rs:948-984.
- C24 PASSTHROUGH commands.rs:132-135 falls through to Action::Submit = child turn; Go tui.go:1174-1181 also submits these as turns.
- C25 /init /review /doctor are turns in Go tui.go:1174-1181 and in Rust Submit fallthrough (earlier "diff" verdict withdrawn).
- C27 BUSY_OK commands.rs:117-128, gate commands.rs:332-334, BUSY_SLASH_MSG identical to tui.go:844.
- C28 unknown command note matches tui.go runSlash default.
- C32 usage overlay swallows keys first mod.rs:658-660; close usage.rs:104-108. No agent-view popup test in Rust.

## remaining diffs
- C10: single record-topic hit relaunches instead of importing; extra session-id route; 30-cap unverified (resume_group/record_turn core).
- C22: /clear goes to engine as raw text; no handoff snapshot (api.rs:1767-1775 handles record split only). core.
- C23: /rotate goes to engine as raw text; no core rotate/handoff. core.
- C30: left click on a panel row does not open agent view; select_press mod.rs:1494-1506 only starts text selection; Go click agentview.go:172 / panelClick agentview.go:160. Needs panel row geometry from render layout.

## out of scope, seen during audit
- /mode <x> and /model <x> are not handled locally in Rust submit; they fall through to Action::Submit and reach the Claude child as raw text. Go: /model switches and rotates (tui.go:1080, repl.go:1385-1395, C07), /mode sets mode (tui.go:1145). Belongs to C07/C18 owner.

## checks
- HOME=$(mktemp -d) cargo test -p chaude-cli -> 226 passed, 0 failed, 1 ignored (run on working tree, which includes another coder's uncommitted mod.rs hunks)
- HOME=$(mktemp -d) cargo test -p chaude-cli version -> 1 passed (tui::commands::tests::version_note_shows_chaude_version)
- cargo clippy --workspace --all-targets -> 0 warnings
- cargo fmt --all --check -> PASS (after rustfmt on commands.rs)

## commit scope
- staged: crates/cli/src/tui/commands.rs (all mine), plan/tui-parity.md (C rows only), crates/cli/src/tui/mod.rs (index blob = HEAD + 2 lines; other coder's hunks not staged), notes/runs/261104-haiku-tui-parity-c.md
- not staged: crates/cli/src/tui/mod.rs working hunks (opus/resident), plan/caveman-structure.md, sessview.rs-E, other notes

## follow-up (C06 C22 C23, C10 C30 already committed)
- C22 fixed (core, api.rs): /clear intercepted in submit; clear_session runs handoff turn with snapshot, then finish_clear restarts child. Go repl.go:1401-1430. Tests: clear_command_carries_snapshot_to_a_fresh_session (engine.rs).
- C23 fixed (core, api.rs): /rotate intercepted in submit; manual_rotate runs start_handoff("manual"). Go repl.go:1231. Test: rotate_command_hands_off_to_a_fresh_session (engine.rs).
- C06 fixed (mod.rs): /status answered locally (Go tui.go:1079-1083), real ctx/threshold/rotations. session id shows "-". Test: status_is_answered_locally_not_sent_to_claude.
- engine.rs session_ids_new_session_and_tool_input_are_emitted: /clear half removed (old contract, timed out). ToolInput/SessionIds assertions kept.
- gaps: parked children (Go endAllParked) not handled in finish_clear; TUI transcript clear on /clear unverified; /status session id "-".
- checks: `HOME=$(mktemp -d) cargo test --workspace` run1 and run2: 636 passed, 0 failed, 64 ignored each. clippy --workspace --all-targets: 0 warnings. cargo fmt --all --check: PASS.
- env: rustup re-sync failed on full disk when HOME was a fresh temp dir; ran with RUSTUP_HOME/CARGO_HOME pointed at real home.
- open `[ ]` rows in plan/tui-parity.md: 56 (I27-I35, C01-05, C07-09, C14-15, C26, E01-13, E16-17, E20-22, E24-28, X01-02, X05-13, A01-02).
