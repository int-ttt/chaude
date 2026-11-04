# engine-tui-data: coder idle expiry and exit handoff (crates/core)

date: 2026-11-04
branch: rust-port
scope: crates/core only (cli/tui owned by another coder)
go_truth: deprecated/src/main/intt/internal/harness/ (idlesweep.go, exit_handoff.go, resident coder)

## commits (local, not pushed)
- 0e5932b feat(core): resident coder slots with stage rows and CoderReply (api.rs, resident.rs)
- ccde40c feat(core): project settings write haiku agents and Agent|Task guard (settings/project.rs, settings/hooks/main-thread-guard.py, settings/agents/)
- this run: coder idle expiry, exit handoff, coder/idle/handoff engine tests, doc comment fix (api.rs, tests/engine.rs, this note)

## done
- prompter run: done before this run (earlier commits)
- resident coder slots: done (Cmd::Coder, extra_coders 0-8, CODER_TIMEOUT 30 min, Stage Working/Done/Failed, CoderReply)
- coder test: done (tests/engine.rs coder_task_emits_working_done_rows_and_reply)
- N14 idle expiry for extra coders: done. Config.coder_idle_ttl default 270 s. Base coder (slot 0) never expires (Go isExtraCoder branch). Wakes through wait_budget, swept in on_quiet.
- exit handoff writer: done. Config.handoff_path (default None). Written after the actor loop, before children are killed (covers Cmd::Shutdown and handle disconnect). Fields: version 1, cwd, main_session (omitted when empty), saved_at_unix.
- settings port (project.rs, agents, hook): committed earlier (ccde40c)
- api.rs misplaced doc comment (coder_role between agent_slot's comments): fixed

## remaining
- resident session ids in handoff: prompter/tui/coder session ids are not recorded (only main_session). Go resident_sessions and owned_sessions are empty.
- handoff read side: Go readExitHandoff (used by /view and /resume on next start) not ported.
- handoff timestamp: saved_at_unix (seconds) instead of Go RFC 3339 time. Change only if a reader needs RFC 3339.
- coder as MCP tool (Go resident_mcp.go): not ported. Rust exposes EngineHandle::coder(task).
- idlesweep main-child parts (childDead, cold cache rotation): not ported. Only the extra coder expiry is.

## gates
- cargo build -p chaude-core -> Finished, no warnings
- cargo test -p chaude-core -> lib 67 passed, engine 42 passed, 0 failed (engine ~40 s)
- cargo fmt -p chaude-core -- --check -> clean (after fmt on crate)
- new engine tests: coder_task_emits_working_done_rows_and_reply, idle_extra_coder_expires_and_base_coder_stays (pid-based, uses kill -0), exit_handoff_records_main_session_on_shutdown

## risks
- idle expiry test relies on kill -0 on a reaped pid after SIGKILL; pid tag is the task name (first = base), so it is deterministic. A flaky 5 s wait would show as "extra coder not expired".
- fake coder sleeps 0.2 s before reply; the back-to-back submit depends on slot 0 still busy at the second submit. Verified passing; no retry added.
- .claude/hooks/main-thread-guard.py (+12 lines) remains uncommitted: outside scope, needs confirmation.
