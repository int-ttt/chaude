// Parity stubs for Go harness group 3 rows that are not ported yet (see plan/rust-test-parity-p3.md).
// Each test is #[ignore]d with its reason and panics when run with --ignored, so it cannot pass by accident.
// Go seams (viaTUI, viaPrompter, residentRepl, fakeSession, spawnSlot, residentSlots, newSessionFn) have no
// Rust counterpart with the same names. cli items are private to crates/cli/src/tui, so they stay
// ignored here until the owning #[cfg(test)] mod (D6) reaches them.

// Go TestRunTurnShowsTUIOutputOnly (resident_pipeline_test.go:88)
#[test]
#[ignore = "parity: with residents the Go pane shows only the tui answer; Rust streams main TextDelta (crates/core/src/api.rs:2205-2211) and cli has no TuiAnswer arm"]
fn go_run_turn_shows_tui_output_only() {
    panic!("not ported: run_turn TUI-only output");
}

// Go TestViewHelpers (resume_test.go:42)
#[test]
#[ignore = "N-A: Go formatTranscript (resume.go:174-175) has no production caller, only resume_test.go:61; list/match/prompts covered by cli sessview::list_match_and_prompts_follow_go_view_helpers"]
fn go_view_helpers() {
    panic!("not ported: sessview helper set (cli-private)");
}
