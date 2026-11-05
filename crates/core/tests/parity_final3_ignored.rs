// Parity stubs for the Go test body parts that the partial rows in plan/rust-test-parity.md still lack
// (final3 workstream). Each test is #[ignore]d with its reason and panics when run with --ignored, so it
// cannot pass by accident. Covered rows were removed and point at their active engine.rs tests. The remaining
// core items need a test seam; cli items live in the bin-only crate (no lib target), so they stay ignored
// here until a cli test seam exists.
// The covered halves of these rows are real tests in parity_final3.rs, parity_final3_resident.rs and
// parity_p12_harness.rs. The MCP spawn-once and created-once rows are converted in parity_final3_resident.rs.

// Go TestIdleSpendRow, background-row write and result_model_usage (idlespend_test.go:46-82)
#[test]
#[ignore = "parity: Go idlespend_test.go:46-82 background row and result_model_usage; Rust api.rs:2991-2994 emits result_model_usage on the background row (D3 I4); the Go row body still needs a test"]
fn go_idle_spend_row_background_row() {
    panic!("not ported: idle spend background row and result_model_usage");
}

// Go TestAgentPromptMissingNotice, notice text and no agent argument (resident_agent_test.go:37-59)
#[test]
#[ignore = "parity: Go resident_agent_test.go:37-59 expects the missing-prompt notice; Rust errors with no notice (parity_harness_p2.rs ~380)"]
fn go_agent_prompt_missing_notice() {
    panic!("not ported: missing agent prompt notice and agent-less spawn");
}

// Go TestIdleControlRequestAnswered, assertions beyond the can_use_tool ask/answer path (idle_test.go:40-53)
#[test]
#[ignore = "parity: Go idle_test.go:40-53 idle canUse between turns; no idle control-request fixture in tests/ (api.rs idle loop)"]
fn go_idle_control_request_answered_rest() {
    panic!("not ported: idle control request remaining assertions");
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
