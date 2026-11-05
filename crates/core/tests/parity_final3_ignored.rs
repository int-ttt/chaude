// Parity stubs for the Go test body parts that the partial rows in plan/rust-test-parity.md still lack
// (final3 workstream). Each test is #[ignore]d with its reason and panics when run with --ignored, so it
// cannot pass by accident. Core items need a Repl/loop seam that is not pub in Rust; cli items live in the
// bin-only crate (no lib target), so they stay ignored here until a cli test seam exists.
// The covered halves of these rows are real tests in parity_final3.rs, parity_final3_resident.rs and
// parity_p12_harness.rs. The MCP spawn-once and created-once rows are converted in parity_final3_resident.rs.

// Go TestIdleSpendRow, background-row write and result_model_usage (idlespend_test.go:46-82)
#[test]
#[ignore = "parity: Go idlespend_test.go:46-82 background row and result_model_usage; Rust api.rs:2506-2508 does not produce result_model_usage"]
fn go_idle_spend_row_background_row() {
    panic!("not ported: idle spend background row and result_model_usage");
}

// Go TestAgentPromptMissingNotice, notice text and no agent argument (resident_agent_test.go:37-59)
#[test]
#[ignore = "parity: Go resident_agent_test.go:37-59 expects the missing-prompt notice; Rust errors with no notice (parity_harness_p2.rs ~380)"]
fn go_agent_prompt_missing_notice() {
    panic!("not ported: missing agent prompt notice and agent-less spawn");
}

// Go TestRegistryNilWhenNoFile, wireClients wiring (cachettl_test.go:23-43)
#[test]
#[ignore = "parity: Go cachettl_test.go:23-43 wireClients wiring; Rust api.rs:773 wires the registry inside api.rs, no test seam"]
fn go_registry_nil_when_no_file_wiring() {
    panic!("not ported: wireClients registry wiring");
}

// Go TestAgentTrackerAndDrain, drain.finished check after the tracker finishes (drain_test.go:11-63)
#[test]
#[ignore = "parity: Go drain_test.go:11-63 feeds a drain tracker that Go production never builds (drain.go:317-327); Rust drains end by drain_timeout (api.rs end_drain)"]
fn go_agent_tracker_drain_finished() {
    panic!("not ported: agent tracker drain.finished");
}

// Go TestIdleControlRequestAnswered, assertions beyond the can_use_tool ask/answer path (idle_test.go:40-53)
#[test]
#[ignore = "parity: Go idle_test.go:40-53 idle canUse between turns; no idle control-request fixture in tests/ (api.rs idle loop)"]
fn go_idle_control_request_answered_rest() {
    panic!("not ported: idle control request remaining assertions");
}

// Go TestRotateKeepsPendingSnapshot (resumeshift_test.go). Rust keeps the pending snapshot on the
// snapshot_kept path (api.rs start_handoff -> rotate_keep_snapshot), so this is a seam gap, not a design difference.
#[test]
#[ignore = "parity: Go resumeshift_test.go rotate with pending snapshot needs a fake newSessionFn seam; not pub in Rust"]
fn go_rotate_keeps_pending_snapshot() {
    panic!("not ported: rotate keeps pending import snapshot via Repl seam");
}

// Go TestStageLinesOrder, full stage order list (resident_agent_test.go:61-102)
#[test]
#[ignore = "parity: Go resident_agent_test.go:61-102 full stage line order; text lives in cli/src/tui/mod.rs:1758, cli is bin-only (no lib target)"]
fn go_stage_lines_full_order() {
    panic!("not ported: stage lines full order");
}

// Go TestInputWidthMixed, cases beyond the mixed-width measure (input_test.go:9-29)
#[test]
#[ignore = "parity: Go input_test.go:9-29 mixed-width input cases; cli is bin-only (no lib target)"]
fn go_input_width_mixed_rest() {
    panic!("not ported: input width mixed remaining cases");
}

// Go TestInputWrapWide, cases beyond the wide-cell wrap (input_test.go:30-51)
#[test]
#[ignore = "parity: Go input_test.go:30-51 wide-cell input wrap cases; cli is bin-only (no lib target)"]
fn go_input_wrap_wide_rest() {
    panic!("not ported: input wrap wide remaining cases");
}

// Go TestSessionPanelAndThinking, panel assertions beyond the thinking row (panel_test.go:17-37)
#[test]
#[ignore = "parity: Go panel_test.go:17-37 session panel and thinking row; cli is bin-only (no lib target)"]
fn go_session_panel_rest() {
    panic!("not ported: session panel assertions");
}

// Go TestEscInterruptClearsForegroundPanel, foreground panel clear (panel_test.go:38-54)
#[test]
#[ignore = "parity: Go panel_test.go:38-54 esc interrupt clears foreground panel; cli is bin-only (no lib target)"]
fn go_esc_interrupt_clears_foreground_panel() {
    panic!("not ported: esc interrupt clears foreground panel");
}

// Go TestPanelMoreRowAndSmallHeight, more-row and small-height panel (panel_test.go)
#[test]
#[ignore = "parity: Go panel_test.go more-row and small-height panel; cli is bin-only (no lib target)"]
fn go_panel_more_row_small_height() {
    panic!("not ported: panel more-row and small height");
}

// Go TestAgentStatusTextPrefixedName, cases beyond the model-prefix skip (panel_test.go:314-339)
#[test]
#[ignore = "parity: Go panel_test.go:314-339 agent status prefixed-name cases; cli is bin-only (no lib target)"]
fn go_agent_status_prefixed_rest() {
    panic!("not ported: agent status prefixed name remaining cases");
}

// Go TestAgentStatusFormat, format cases beyond the status text (panel_test.go:340-366)
#[test]
#[ignore = "parity: Go panel_test.go:340-366 agent status format cases; cli is bin-only (no lib target)"]
fn go_agent_status_format_rest() {
    panic!("not ported: agent status format remaining cases");
}
