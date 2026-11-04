// Parity stubs for the Go test body parts that the partial and missing rows in plan/rust-test-parity.md still
// lack (final3 workstream). Each test is #[ignore]d with its reason and panics when run with --ignored, so it
// cannot pass by accident. Core items need a Repl/loop seam that is not pub in Rust; cli items live in the
// bin-only crate (no lib target), so they stay ignored here until a cli test seam exists.
// The covered halves of these rows are real tests in parity_final3.rs and parity_p12_harness.rs.

// Go TestResidentMCPConfigWritten, "created once" check (resident_coder_test.go)
#[test]
#[ignore = "parity: residentMCPConfig created-once check is Repl-level; not pub in Rust"]
fn go_resident_mcp_config_created_once() {
    panic!("not ported: resident mcp.json created once");
}

// Go TestMainStrictMCP, spawn-once check (resident_agent_test.go)
#[test]
#[ignore = "parity: main child spawned once with --mcp-config is Repl start(); not pub in Rust"]
fn go_main_strict_mcp_spawn_once() {
    panic!("not ported: main child spawned once with --mcp-config");
}

// Go TestIdleSpendRow, background-row write and result_model_usage (idlespend_test.go)
#[test]
#[ignore = "parity: background row write and result_model_usage are record_turn-level, not pub in Rust"]
fn go_idle_spend_row_background_row() {
    panic!("not ported: idle spend background row and result_model_usage");
}

// Go TestAgentPromptMissingNotice, notice text and no agent argument (resident_agent_test.go)
#[test]
#[ignore = "parity: spawnSlot missing-prompt notice and agent-less start are Repl-level, not pub in Rust"]
fn go_agent_prompt_missing_notice() {
    panic!("not ported: missing agent prompt notice and agent-less spawn");
}

// Go TestRegistryNilWhenNoFile, wireClients wiring (cachettl_test.go)
#[test]
#[ignore = "parity: Repl.wireClients (reg, children, ttl, idleRotate) stays in api.rs; not pub in Rust"]
fn go_registry_nil_when_no_file_wiring() {
    panic!("not ported: wireClients registry wiring");
}

// Go TestAgentTrackerAndDrain, drain.finished check after the tracker finishes (drain_test.go)
#[test]
#[ignore = "parity: agent drain.finished flag is set by the Repl drain loop; not pub in Rust"]
fn go_agent_tracker_drain_finished() {
    panic!("not ported: agent tracker drain.finished");
}

// Go TestIdleControlRequestAnswered, assertions beyond the can_use_tool ask/answer path (idle_test.go)
#[test]
#[ignore = "parity: remaining idle control-request assertions live in the Repl idle loop; not pub in Rust"]
fn go_idle_control_request_answered_rest() {
    panic!("not ported: idle control request remaining assertions");
}

// Go TestRotateKeepsPendingSnapshot (resumeshift_test.go). Rust keeps the pending snapshot on the
// snapshot_kept path (api.rs start_handoff -> rotate_keep_snapshot), so this is a seam gap, not a design difference.
#[test]
#[ignore = "parity: Repl rotate with pending snapshot needs a fake newSessionFn seam; not pub in Rust"]
fn go_rotate_keeps_pending_snapshot() {
    panic!("not ported: rotate keeps pending import snapshot via Repl seam");
}

// Go TestStageLinesOrder, full stage order list (resident_agent_test.go)
#[test]
#[ignore = "parity: TUI stage lines order, cli is bin-only (no lib target)"]
fn go_stage_lines_full_order() {
    panic!("not ported: stage lines full order");
}

// Go TestInputWidthMixed, cases beyond the mixed-width measure (input_test.go)
#[test]
#[ignore = "parity: TUI input width cases, cli is bin-only (no lib target)"]
fn go_input_width_mixed_rest() {
    panic!("not ported: input width mixed remaining cases");
}

// Go TestInputWrapWide, cases beyond the wide-cell wrap (input_test.go)
#[test]
#[ignore = "parity: TUI input wrap cases, cli is bin-only (no lib target)"]
fn go_input_wrap_wide_rest() {
    panic!("not ported: input wrap wide remaining cases");
}

// Go TestSessionPanelAndThinking, panel assertions beyond the thinking row (panel_test.go)
#[test]
#[ignore = "parity: TUI session panel, cli is bin-only (no lib target)"]
fn go_session_panel_rest() {
    panic!("not ported: session panel assertions");
}

// Go TestEscInterruptClearsForegroundPanel, foreground panel clear (panel_test.go)
#[test]
#[ignore = "parity: TUI foreground panel clear on esc, cli is bin-only (no lib target)"]
fn go_esc_interrupt_clears_foreground_panel() {
    panic!("not ported: esc interrupt clears foreground panel");
}

// Go TestPanelMoreRowAndSmallHeight, more-row and small-height panel (panel_test.go)
#[test]
#[ignore = "parity: TUI panel more-row and small height, cli is bin-only (no lib target)"]
fn go_panel_more_row_small_height() {
    panic!("not ported: panel more-row and small height");
}

// Go TestAgentStatusTextPrefixedName, cases beyond the model-prefix skip (panel_test.go)
#[test]
#[ignore = "parity: TUI agent status text cases, cli is bin-only (no lib target)"]
fn go_agent_status_prefixed_rest() {
    panic!("not ported: agent status prefixed name remaining cases");
}

// Go TestAgentStatusFormat, format cases beyond the status text (panel_test.go)
#[test]
#[ignore = "parity: TUI agent status format cases, cli is bin-only (no lib target)"]
fn go_agent_status_format_rest() {
    panic!("not ported: agent status format remaining cases");
}
