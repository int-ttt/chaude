// Parity stubs for Go harness group 3 rows that are not ported yet (see plan/rust-test-parity-p3.md).
// Each test is #[ignore]d with its reason and panics when run with --ignored, so it cannot pass by accident.
// Go seams (viaTUI, viaPrompter, residentRepl, fakeSession, spawnSlot, residentSlots, newSessionFn) have no
// Rust counterpart with the same names. cli items live in a bin-only crate (no lib target), so they stay
// ignored here until a cli test seam exists.

// Go TestRunTurnShowsTUIOutputOnly (resident_pipeline_test.go:88)
#[test]
#[ignore = "parity: with residents the Go pane shows only the tui answer; Rust streams main TextDelta (crates/core/src/api.rs:2205-2211) and cli has no TuiAnswer arm"]
fn go_run_turn_shows_tui_output_only() {
    panic!("not ported: run_turn TUI-only output");
}

// Go TestSpawnSlotParksReadingChild (resident_route_test.go:55)
#[test]
#[ignore = "parity: resident slots have no parked or idle-reader state (crates/core/src/api.rs:1672-1680 spawn_residents)"]
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
