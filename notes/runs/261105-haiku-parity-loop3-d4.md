status: done
scope: D4 items I5-I9 (crates/core)
commits: abba091 (I5), 9e3ac71 (I6), 47a8423 (I7), 10e0967 (I8), ca43995 (I9)
checks: per item fmt PASS, clippy 0 warnings, test-isolated 662/663/664/665/666 passed, 0 failed, 61 ignored
retries: I6 1 (clippy), I8 2 (fixture switch, fmt hunks reverted), others 0
changed: crates/core/src/api.rs (I6 spawn_residents, I9 wire_clients), crates/core/tests/engine.rs (I5-I9 tests)
not done: none in D4 scope. #[ignore] stubs left in place (D6 removes). Pre-existing dirty files left unstaged.
workspace: 3.2G (no cargo clean)
doubts: go_rotate_keeps_pending_snapshot, go_send_resident_respawns_dead_slot, go_resident_spawns_eager, go_resident_slots_four, go_stage_lines_full_order, go_idle_control_request_answered_rest, go_registry_nil_when_no_file_wiring stubs still #[ignore]; the Rust tests covering their behavior live in engine.rs and are not yet linked from the stubs.
