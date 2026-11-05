task: item4 ignored-test wiring, round triage
commits: none
changed_paths: none (notes only)
tests_unignored: 0
tests_still_ignored: 5 (p2:379, p2:404, p3_ignored:9, :16, :23)

p2:379 agent_prompt_missing_gives_korean_notice_and_default_session
  go: deprecated/src/main/intt/internal/harness/resident_agent_test.go:37-59 (spawnSlot notice "에이전트 규칙 없음(haiku-missing): 기본 세션으로 시작", spawn without agent)
  rust: crates/core/src/resident.rs:47-61 agent_body returns Err("no agent file ...") English only; no spawn_slot seam (api.rs spawn_tui/spawn_coder ~1571/1644 take no prompter slot, no newSessionFn-style injection)
  verdict: B. needs spawn seam + Korean notice + default-session path. not minimal.

p2:404 idle_spend_row_counts_subagent_tokens_and_spawns
  go: deprecated/src/main/intt/internal/harness/idlespend_test.go:46-82 (r.idleEvent -> bg row jsonl with sub_output_tokens, spawns, result_model_usage)
  rust: TurnSpend exists (crates/core/src/turn_spend.rs, sub_output_tokens/spawns/spawn_types OK, unit-tested), merged only into turn row via with_spend (api.rs:3691, metrics.log at api.rs:2923).
  missing: no idle/bg row writer in Rust; no result_model_usage from result.modelUsage (only "{}" fixture at record_turn.rs:256).
  verdict: B. needs bg-row path + modelUsage port + engine-level test body.

p3_ignored:9 go_via_tui_failure_notice
  go: resident_pipeline_test.go:63-84 (viaTUI error notice, timeout notice "시간초과")
  rust: start_tui api.rs:1545-1562 emits tui_notice on spawn error (resident.rs:161). present.
  missing: timeout half needs configurable TUI_TIMEOUT (resident.rs:14 const 120s); test would wait 120s without seam. error half needs full engine turn driven via fake claude.
  verdict: B (partial feature present). not minimal.

p3_ignored:16 go_run_turn_shows_tui_output_only
  go: resident_pipeline_test.go:88+ (residentRepl, pane shows only tui answer)
  rust: no residentRepl seam. verdict: B.

p3_ignored:23 go_send_resident_respawns_dead_slot
  go: resident_respawn_test.go:11 (dead slot respawn)
  rust: no respawn hook in resident.rs (audit says so). verdict: B.

ws: du -sk target = 2.8GB (<13GB), no cargo clean.
checks: none run (no code change). 
risk: none to tree. next round needs design call: spawn seam (prompter/tui slot injection) and configurable TUI_TIMEOUT.
