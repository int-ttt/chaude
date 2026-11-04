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

// Go TestSendResidentRespawnsDeadSlot (resident_respawn_test.go)
#[test]
#[ignore = "parity: resident dead-slot respawn not wired in Rust (no respawn hook in resident.rs)"]
fn go_send_resident_respawns_dead_slot() {
    panic!("not ported: resident respawn of a dead slot");
}

// Go TestViaPrompterRawWhenRespawnFails (resident_respawn_test.go)
#[test]
#[ignore = "parity: respawn-failure raw fallback path not wired in Rust (fallback exists for prompter failure only)"]
fn go_via_prompter_raw_when_respawn_fails() {
    panic!("not ported: raw prompt when respawn fails");
}

// Go TestSpawnSlotParksReadingChild (resident_route_test.go:55)
#[test]
#[ignore = "parity: spawnSlot seam not wired in Rust; parked reading child has no Rust equivalent"]
fn go_spawn_slot_parks_reading_child() {
    panic!("not ported: spawn_slot parks reading child");
}

// Go TestResidentSpawnsEager (resident_test.go)
#[test]
#[ignore = "parity: eager resident spawn not exposed by a pub core API"]
fn go_resident_spawns_eager() {
    panic!("not ported: eager resident spawn");
}

// Go TestResidentSlotsFour (resident_test.go)
#[test]
#[ignore = "parity: resident slot count (4) not exposed by a pub core API; Config::default extra_coders is in parity_harness_p3.rs"]
fn go_resident_slots_four() {
    panic!("not ported: resident slot count check");
}

// Go TestViewHelpers (resume_test.go:42)
#[test]
#[ignore = "parity: cli-private sessview helpers; bin-only crate has no lib target for integration tests"]
fn go_view_helpers() {
    panic!("not ported: sessview helper set (cli-private)");
}

// Go TestRotateKeepsPendingSnapshot (resumeshift_test.go:65)
#[test]
#[ignore = "parity: model switch must keep pending import snapshot; rotate path not wired to the snapshot in Rust"]
fn go_rotate_keeps_pending_snapshot() {
    panic!("not ported: rotate keeps pending import snapshot");
}

// Go TestReleaseCopiesSelection (selection_test.go:50)
#[test]
#[ignore = "parity: clipboard copy on release is cli-private; bin-only crate has no lib target"]
fn go_release_copies_selection() {
    panic!("not ported: release copies selection (cli-private)");
}

// Go TestViewPromptScrollsInsideTranscript (sessview_test.go:139)
#[test]
#[ignore = "parity: sessview prompt scroll is cli-private; bin-only crate has no lib target"]
fn go_view_prompt_scrolls_inside_transcript() {
    panic!("not ported: sessview prompt scroll (cli-private)");
}

// Go TestFmtK (unit_test.go:133)
#[test]
#[ignore = "parity: fmt_k is private in crates/cli/src/tui/bar.rs:90; bin-only crate has no lib target"]
fn go_fmt_k() {
    panic!("not ported: fmtK/fmtLimit cases (cli-private fmt_k)");
}

// Go TestCtxBar (unit_test.go:149)
#[test]
#[ignore = "parity: ctx bar helper is cli-private; bin-only crate has no lib target"]
fn go_ctx_bar() {
    panic!("not ported: ctx bar (cli-private)");
}
