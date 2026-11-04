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

## sub-group 3a: Config.residents default (Go repl.go:1354 always creates resident slots)
- api.rs Config::default residents: false -> true
- engine.rs config() helper pins residents: false (baseline event-stream tests unchanged); stage test opts in
- gates: clippy -D warnings PASS; cargo test -p chaude-core PASS (58 unit + 35 engine); fmt --check PASS; cargo build --workspace PASS
- open: settings key. Go has no residents toggle; the only related key is extra_coders (0-8, settings_picker.go:74-99), not yet ported.
- remaining: prompter wiring into submit, tui/coder slots, Failed producer, N14 idlesweep, exit_handoff.go

## sub-group 4a: SpawnOpts.shape (Go sessionArgs resident shape)
- proc.rs: Shape {minimal, tools, strict_mcp, no_slash, rule}; SpawnOpts.shape; build_args(model, resume, &Shape) emits Go order
- test resident_shape_args_match_go_session_args (minimal and coder argv tails)
- gates: clippy PASS; cargo test -p chaude-core PASS; fmt PASS; build --workspace PASS
- remaining: prompter child wiring (b), coder+tui slots (c), N14 (d), exit_handoff (e)

## sub-group 4b: prompter child (Go viaPrompter)
- resident.rs: PROMPTER_RULE = include_str!(rules/resident-prompter.md) (verbatim Go rule); Prompter::fail(reason); unit test fail_passes_input_through_with_reason
- api.rs: Input::Prompter(epoch, ProcMsg); State prompter / prompter_proc / prompter_epoch / prompter_deadline
- submit: queues while busy or prompter waiting; begin_turn -> start_prompter (residents on) or dispatch (off)
- prompter child: haiku, Shape minimal + rule, spawned on first prompt, kept alive; result line = reply
- finish_prompter: stage prompter Done/Failed from outcome, then dispatch the outcome text (summary+original, or raw)
- Failed producers: spawn error, start error, write error, child exit (Eof), timeout (PROMPTER_TIMEOUT 60 s via on_quiet/wait_budget)
- deferred rotation does not run while a prompter wait is pending
- tests/engine.rs: fake() now plays prompter role when argv has system-prompt (reply from prompter.jsonl; empty file = exit). stage test expects prompter Working/Done rows first.
- new tests: prompter_summary_wraps_prompt_for_main_child, prompter_failure_falls_back_to_raw_prompt
- gates: cargo test -p chaude-core 60 lib + 37 engine PASS; clippy --all-targets -D warnings PASS; fmt --check PASS; cargo build --workspace PASS
- not tested: 60 s prompter timeout path (too slow for suite); queued second prompt during prompter wait
- doubt: queued merged turns (next_turn) still bypass the prompter, as before
- remaining: coder+tui slots (c), N14 idlesweep (d), exit_handoff (e)

## sub-group 4c: tui child (Go viaTUI, resident.go:354-380)
- resident.rs: TUI_TIMEOUT 120 s; tui_prompt(reply) (Go prompt text verbatim); agent_body(cwd, name) (cwd then $HOME agents dir, Go agentBody); strip_frontmatter (Go stripFrontmatter); unit tests strip_frontmatter_drops_header_block, tui_prompt_wraps_reply_after_instruction
- api.rs: UiEvent::TuiAnswer { text } (new non_exhaustive variant); Input::Tui(epoch, ProcMsg); State tui_proc / tui_epoch / tui_deadline
- end_turn: after remember_turn, residents on and non-empty reply -> start_tui (turn stays busy); tail moved to after_turn (rotation, then next_turn)
- start_tui: stage tui Working, write tui_prompt(reply); spawn_tui = haiku, Shape minimal + rule = haiku-tui agent body (cwd, then HOME); epoch-tagged child kept alive
- answer: result line trimmed -> stage tui Done + TuiAnswer; empty -> Failed("tui empty reply"); exit -> Failed("tui exited"); timeout (TUI_TIMEOUT via wait_budget/on_quiet) -> Failed("tui timed out"); spawn/write errors -> Failed; main reply kept (no TuiAnswer)
- interrupt while tui runs is not sent to the idle main child (avoids stale interrupted_at)
- tests/engine.rs: fake() writes .claude/agents/haiku-tui.md (TUIMARK) and tui.jsonl; argv TUIMARK selects tui role, tui.log records its input. Residents tests set cfg.cwd = fake dir.
- new tests: tui_answer_replaces_main_reply_after_turn (text_turn.jsonl), tui_exit_keeps_main_reply_and_reports_failed_stage
- note: resume_agent_turn.jsonl has no text deltas, so the stage test sees no tui rows (no reply, no tui run)
- gates: cargo test -p chaude-core 62 lib + 39 engine PASS; clippy --all-targets -D warnings PASS; fmt --check PASS; cargo build --workspace PASS
- not tested: 120 s timeout path (too slow); Result-only replies (no text deltas) skip the tui because turn_reply is delta-built
- doubt: TUI consumer (replace shown main text on TuiAnswer) lives in crates/cli (other agent)
- remaining: coder slot (c), N14 idlesweep (d), exit_handoff (e)
