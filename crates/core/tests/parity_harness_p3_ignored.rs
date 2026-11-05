// Parity stubs for Go harness group 3 rows that are not ported yet (see plan/rust-test-parity-p3.md).
// Each test is #[ignore]d with its reason and panics when run with --ignored, so it cannot pass by accident.
// Go seams (viaTUI, viaPrompter, residentRepl, fakeSession, spawnSlot, residentSlots, newSessionFn) have no
// Rust counterpart with the same names. cli items live in a bin-only crate (no lib target), so they stay
// ignored here until a cli test seam exists.

// Go TestViaTUIFailureNotice (resident_pipeline_test.go)
#[test]
#[ignore = "parity: resident TUI translator failure notice seam not wired in Rust (api.rs in progress)"]
fn go_via_tui_failure_notice() {
    panic!("not ported: resident TUI translator failure notice");
}

// Go TestRunTurnShowsTUIOutputOnly (resident_pipeline_test.go)
#[test]
#[ignore = "parity: run_turn TUI-only output seam (residentRepl) not wired in Rust"]
fn go_run_turn_shows_tui_output_only() {
    panic!("not ported: run_turn TUI-only output");
}

// Go TestSpawnSlotParksReadingChild (resident_route_test.go:55)
#[test]
#[ignore = "parity: spawnSlot seam not wired in Rust; parked reading child has no Rust equivalent"]
fn go_spawn_slot_parks_reading_child() {
    panic!("not ported: spawn_slot parks reading child");
}

// Go TestViewHelpers (resume_test.go:42)
#[test]
#[ignore = "parity: formatTranscript has no Rust counterpart (Go resume.go:175); partial cli test sessview::list_match_and_prompts_follow_go_view_helpers"]
fn go_view_helpers() {
    panic!("not ported: sessview helper set (cli-private)");
}

// Go TestReleaseCopiesSelection (selection_test.go:50)
#[test]
#[ignore = "parity: copy_text hard-wires native clipboard (crates/cli/src/tui/mod.rs:1952), no injectable hook; partial cli test tui::release_returns_the_dragged_text_for_copy"]
fn go_release_copies_selection() {
    panic!("not ported: release copies selection (cli-private)");
}
