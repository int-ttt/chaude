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
