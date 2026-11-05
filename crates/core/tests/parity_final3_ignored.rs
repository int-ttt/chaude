// Parity stubs for the Go test body parts that the partial rows in plan/rust-test-parity.md still lack
// (final3 workstream). Each test is #[ignore]d with its reason and panics when run with --ignored, so it
// cannot pass by accident. Covered rows were removed and point at their active tests. The remaining
// core items need a test seam. cli items are covered by #[cfg(test)] mods in crates/cli/src/tui (private access);
// the cli rows left here are the ones those mods do not reach yet.
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

// Go TestInputWrapWide, Up-key row move after the wrap (input_test.go:43-50). The wrap and cursor part is
// covered by tui::tests::input_wraps_wide_cells_and_places_cursor. Up calls Editor::move_row, which moves
// logical lines (mod.rs Up key, edit.rs move_row), while Go MoveRow (input.go:185) moves visual rows.
#[test]
#[ignore = "parity: Up key moves logical lines (edit.rs move_row), Go MoveRow moves visual rows (input.go:185); divergence, not a test gap"]
fn go_input_wrap_wide_rest() {
    panic!("not ported: input wrap wide Up-key visual row move");
}

// Go TestAgentStatusTextPrefixedName, opusRunning on the prefixed opus-architect agent (panel_test.go:314-339).
// The status text and model-prefix skip are covered by tui::bar tests (agent_name_with_model_prefix_skips_model_segment).
#[test]
#[ignore = "parity: Go panel_test.go:330-337 opusRunning for a prefixed opus-architect agent is not asserted in cli tests"]
fn go_agent_status_prefixed_rest() {
    panic!("not ported: opusRunning on prefixed agent name");
}
