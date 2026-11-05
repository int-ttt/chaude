# ignore audit (read-only)
total: 27 real #[ignore] (excludes 4 non-test hits: p3:2 comment, p12:7 comment, final3:2 comment, p12:22 macro template)
A: 1 (mod.rs:4609 bench, no Go counterpart)
B: 26
C: 0
rule: A needs platform/Go-only/network/terminal reason; reasons here are "not wired in Rust" or "cli bin-only, no lib target" (test-seam, portable Go behavior)

| file:line | test | class | Go cite check |
|---|---|---|---|
| parity_harness_p3_ignored.rs:9 | go_via_tui_failure_notice | B | resident_pipeline_test.go:63 OK |
| parity_harness_p3_ignored.rs:16 | go_run_turn_shows_tui_output_only | B | resident_pipeline_test.go:88 OK |
| parity_harness_p3_ignored.rs:23 | go_send_resident_respawns_dead_slot | B | resident_respawn_test.go:11 OK |
| parity_harness_p3_ignored.rs:30 | go_via_prompter_raw_when_respawn_fails | B | resident_respawn_test.go:46 OK |
| parity_harness_p3_ignored.rs:37 | go_spawn_slot_parks_reading_child | B | resident_route_test.go:55 OK |
| parity_harness_p3_ignored.rs:44 | go_resident_spawns_eager | B | resident_test.go:27 OK |
| parity_harness_p3_ignored.rs:51 | go_resident_slots_four | B | resident_test.go:44 OK |
| parity_harness_p3_ignored.rs:58 | go_view_helpers | B | resume_test.go:42 (name match) |
| parity_harness_p3_ignored.rs:65 | go_release_copies_selection | B | selection_test.go:50 OK |
| parity_harness_p3_ignored.rs:72 | go_view_prompt_scrolls_inside_transcript | B | sessview_test.go:139 OK |
| parity_harness_p3_ignored.rs:79 | go_fmt_k | B | unit_test.go:133 OK; Go fmtK repl.go:142 |
| parity_harness_p3_ignored.rs:86 | go_ctx_bar | B | unit_test.go:149 OK |
| parity_harness_p2.rs:379 | (missing agent file notice) | B | Go resident_agent_test.go:37-59 OK |
| parity_harness_p2.rs:404 | idle_spend_row_counts_subagent_tokens_and_spawns | B | idlespend_test.go:46-82 OK |
| parity_final3_ignored.rs:10 | idlespend row | B | idlespend_test.go:46-82 OK (api.rs:2506-2508 not verified) |
| parity_final3_ignored.rs:17 | missing-prompt notice | B | resident_agent_test.go:37-59 OK |
| parity_final3_ignored.rs:24 | cachettl wiring | B | cachettl_test.go:23-43 OK |
| parity_final3_ignored.rs:31 | drain tracker | B | drain_test.go:11-63 OK |
| parity_final3_ignored.rs:38 | idle canUse | B | idle_test.go:40-53 OK |
| parity_final3_ignored.rs:46 | rotate pending snapshot | B | resumeshift_test.go:65 (no line cited in reason) |
| parity_final3_ignored.rs:53 | stage line order | B | resident_agent_test.go:61-102 OK |
| parity_final3_ignored.rs:60 | input wide wrap | B | input_test.go:30-51 OK |
| parity_final3_ignored.rs:67 | session panel/thinking | B | panel_test.go:17-37 OK |
| parity_final3_ignored.rs:74 | panel more-row | B | panel_test.go:100 OK |
| parity_final3_ignored.rs:81 | agent status prefixed | B | panel_test.go:314-339 OK |
| parity_final3_ignored.rs:88 | agent status format | B | panel_test.go:340-366 OK |
| crates/cli/src/tui/mod.rs:4609 | bench timing run | A | bench only, no Go parity behavior (own reason) |
