# engine-tui-data: sub-group status (coder 2)
date: 261104
agent: coder2
branch: rust-port

done:
- d692c2a: abandon_turn ends only foreground agents (test abandoned_turn_ends_foreground_agent_but_not_background_agent)
- d7fea5d: N07 handoff snapshot lists running background agents (Go bgHandoffLines), agent_info map, yymmdd_today UTC (test handoff_snapshot_lists_running_background_agent_with_result_path)
- gates at d7fea5d: cargo test chaude-core 50 lib + 32 engine pass; clippy -D warnings clean; fmt --check clean

remaining:
- resident.rs + Stage UiEvent (B13 E25 E27 X05): too big for this pass, see 261104-engine-tui-data-e-stage-blocker.md
- N14 idle sweep: partly covered by b38f9a1; rest of Go idlesweep.go:32 not ported
- exit_handoff.go (149 lines) port: not started
- N07 done-agent lines: need result-file writer (agentresult.go appendAgentResult), not ported
- notes/runs/261104-engine-tui-data-core-groups.md and notes/runs/engine-tui-data.md: not yet updated or committed

## priority 1 (coordinator): resumed agents
- 71586bf: agentId parsed from async launch result (agent_ids map). SendMessage "Resuming agent <id>" or resumedAgentId re-activates the agent under its tool id and emits AgentResumed { id, description, started_at }. <task-notification> string user message ends named agents via finish_agent (AgentEnd).
- test: resumed_background_agent_is_active_again_until_task_notification (fixture resume_agent_turn.jsonl) PASS
- gates at 71586bf: core test 50 lib + 33 engine PASS; clippy -D warnings PASS; fmt --check PASS; cargo build --workspace PASS
- /view: cli agentview.rs reads its own AgentRec (cli records), not core UiEvent; no core change needed.
- doubt: the "vanish" root cause is not reproduced. Async path never removed the agents entry; the fix covers resume and notification ends only. Real SendMessage stream text is unverified (format from Go testdata bg_turn_live.jsonl).

## af3a743 and resident sub-group 1 (coordinator)
- af3a743: resume reads full resumedAgentId from SendMessage JSON; success:false ignored (fixture resume_agent_turn.jsonl updated)
- resident sub-group 1: crates/core/src/resident.rs (new, `pub mod resident`): StagePhase {Working, Done, Failed(String)}, PrompterOutcome {text, stage}, Prompter state machine (start/reply/timeout, one pending summary), PROMPTER_TIMEOUT 60s, parse_prompter_reply, need_original (hand port of Go needOriginal, path check is not a regex)
- api.rs UiEvent::Stage { slot: String, phase: crate::resident::StagePhase } (after ModeChanged). No producer yet; cli matches use wildcard.
- tests: 8 resident unit tests PASS
- gates: cargo test chaude-core 58 lib + 33 engine PASS; clippy --all-targets -D warnings PASS; fmt --check PASS; cargo build --workspace PASS (27 cli dead-code warnings from other in-progress work, none from Stage)
- remaining sub-groups 2+: tui/coder slots (residentSlots, residentMaxWorkers 8, extraCoderTTL 270s), socket IPC, prompter routing in send path (viaPrompter stage flow), tui translation (viaTUI, 120s), Stage producers
- doubt: need_original path rule is a hand approximation of Go regex; edge cases may differ

## resident sub-group 2a: stage producers (coordinator)
- item 1 verified: af3a743 (resumedAgentId JSON parser + fixture) committed; present on rust-port
- Go gate: repl.go emits stage rows only when len(r.resident) > 0. Rust mirrors it with Config.residents (default false).
- producers: Stage{slot:"main", Working} on successful send_turn; Stage{slot:"main", Done} in end_turn before TurnEnd; Stage{slot:"agent:<description>", Working} on AgentStart; Stage{slot:"agent:<description>", Done} in finish_agent and abandon_turn.
- tests: resident_stage_rows_bracket_main_turn_and_agent_calls (exact sequence on resume_agent_turn.jsonl), no_stage_rows_without_residents
- gates: cargo test chaude-core 58 lib + 35 engine PASS; clippy --all-targets -D warnings PASS; fmt --check PASS; cargo build --workspace PASS
- NOT done: coder and tui child slots (second claude child, Input tag per slot, IPC), prompter wiring into submit (Prompter state machine has no caller yet), tui translation (viaTUI), coder flow, Failed phase producer (no failing path mapped yet), resumed-agent Working row (AgentResumed not staged), N14 idle sweep, exit_handoff.go port
