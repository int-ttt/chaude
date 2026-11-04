# test-parity final3 convert (261104)

status: done
scope: 14 partial rows (stubs in crates/core/tests/parity_final3_ignored.rs)
branch: rust-port (no push)

## converted (2 rows, now covered)
- TestMainStrictMCP -> crates/core/tests/parity_final3_resident.rs::main_spawned_once_with_strict_mcp
  - engine start, one main spawn with --mcp-config and --strict-mcp-config, no --disable-slash-commands, still one spawn after a turn
  - Go: resident_agent_test.go:195-222
- TestResidentMCPConfigWritten -> crates/core/tests/parity_final3_resident.rs::mcp_config_path_reused_after_respawn
  - idle_rotate 1s forces a respawn, same --mcp-config path on both spawns
  - Go: resident_coder_test.go:214-240
- old stubs go_main_strict_mcp_spawn_once and go_resident_mcp_config_created_once removed (converted, not kept)

## kept ignored (12 partial rows, reasons now carry Go file:line)
- TestIdleSpendRow result_model_usage: Go idlespend_test.go:46-82; Rust api.rs:2506-2508 does not produce result_model_usage (real difference)
- TestAgentPromptMissingNotice: Go resident_agent_test.go:37-59; Rust errors with no notice (real difference, parity_harness_p2.rs ~380)
- TestRegistryNilWhenNoFile wiring: Go cachettl_test.go:23-43; wiring in api.rs:773, no test seam
- TestAgentTrackerAndDrain drain.finished: Go drain_test.go:11-63; no drain type (api.rs:322) (real difference)
- TestIdleControlRequestAnswered rest: Go idle_test.go:40-53; no idle fixture
- TestStageLinesOrder full order: Go resident_agent_test.go:61-102; text in cli/src/tui/mod.rs:1758 (bin-only)
- TestInputWidthMixed rest: Go input_test.go:9-29 (bin-only)
- TestInputWrapWide rest: Go input_test.go:30-51 (bin-only)
- TestSessionPanelAndThinking rest: Go panel_test.go:17-37 (bin-only)
- TestEscInterruptClearsForegroundPanel: Go panel_test.go:38-54 (bin-only)
- TestAgentStatusTextPrefixedName rest: Go panel_test.go:314-339 (bin-only)
- TestAgentStatusFormat rest: Go panel_test.go:340-366 (bin-only)

## counts (plan/rust-test-parity.md, recounted by awk on status column)
- covered 235, partial 12, ignored 48, missing 0, n-a 25, total 320

## checks
- cargo test -p chaude-core --test parity_final3 --test parity_final3_resident --test parity_final3_ignored
  - parity_final3: 7 passed, 0 failed
  - parity_final3_resident: 2 passed, 0 failed
  - parity_final3_ignored: 0 passed, 14 ignored
- HOME=$(mktemp -d) CARGO_HOME=/Users/leekwangseok/.cargo RUSTUP_HOME=/Users/leekwangseok/.rustup cargo test --workspace
  - all binaries 0 failed; passed sum 627 (chaude_clean 11, chaude 224, chaude_core lib 232, engine 68, final2 1, final3 7, final3_resident 2, p1 3, p2 9, p3 1, p3_spend 9, p12 9, record 3, gui 2, scripts 3, charcount 5, smoke 7, update lib 15, parity_update 16)
- cargo clippy --workspace --all-targets: exit 0, 0 warning/error lines
- cargo fmt --all -- --check: exit 0 (after rustfmt on parity_final3_resident.rs)

## notes
- first run of mcp_config_path_reused_after_respawn timed out (idle rotation never fired): fake turn had no assistant usage, and api.rs idle_left needs last_api (set only on usage). Fixed by adding usage to the fake assistant line.
- new file parity_final3_resident.rs instead of appending to parity_final3.rs (own HOME/record env setup, LOCK, WAIT).
- engine tests run in one process per binary; HARNESS_RECORD_DIR set to temp dir via Once.
- other coders' uncommitted cli/tui and notes changes were not touched.
