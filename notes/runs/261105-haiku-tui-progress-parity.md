task: tui-progress-parity (Rust TUI agent/progress lines vs Go, Go = truth)
sample: record/pastes/2bf311ec.md
commit: 0b92190 fix(tui): match Go agent progress lines (label, order, done notice)
changed:
  - crates/core/src/api.rs: agent_types map (Go subagentLabel); "· agent <label> 호출" before ToolStart; tool_result emits 완료 + AgentEnd before ToolResult (foreground); AgentEnd gets name/task; interrupt path drops stage 완료 (Go finishForeground emits no stage row)
  - crates/core/tests/engine.rs: AgentEnd literal -> match; stage slot expectation agent:scan -> agent:general (subagent_type)
  - crates/cli/src/tui/mod.rs: removed "◆ agent started", "◆ agent done in", TurnStats [측정] transcript row (record file keeps it); done notice = done_line(failed, model, name, task); agent_outcome -> HashSet; tests updated + agent_stage_and_done_follow_go_labels added
checks:
  - cargo fmt --all -- --check: PASS (exit 0; fmt touched only the 3 files)
  - cargo clippy -p chaude-core -p chaude-cli --all-targets -- -D warnings: PASS (exit 0)
  - cargo test -p chaude-core -p chaude-cli: PASS (exit 0; cli 258+4 ok, core engine 97 ok, lib 243 ok)
  - cargo build -p chaude-cli --bin chaude: PASS -> target/debug/chaude rebuilt
doubts:
  - done notice "what" = task (Go: Summary, else lastSummary(Log), else Task). Rust live path does not track the sub-agent's last text line; sample shows task, so task used.
  - background launch: stage 완료 timing not changed (Go emits 완료 at launch tool_result; Rust at the agent's end).
  - stage 호출 order (before ● Agent line) is enforced in code; no test asserts relative order to ToolStart.
  - [측정] absence in Go TUI: inferred from repl.go:1143 writing to os.Stdout under bubbletea; not run in a live TTY.
  - cli interrupt fallback (finish_foreground) uses description for name and task; rare, core AgentEnd normally arrives first.

## follow-up: done notice what = subagent last text (2535379)
done: Go doneLine what = Summary, else lastSummary(Log), else Task. Rust live path now tracks last plain text line per agent.
commit: 2535379 fix(core): agent done notice uses the subagent's last text line
changed:
  - crates/core/src/api.rs: AgentInfo.last_text + done_what(); on_sub_agent_line keeps last_summary(batch); finish_agent and abandon_turn send AgentEnd.summary
  - crates/core/src/api.rs: UiEvent::AgentEnd field task -> summary (Go doneLine what)
  - crates/core/tests/engine.rs: agent_end_summary_is_last_subagent_text_else_task (subagent text "found 3 files"; opus fixture falls back to task "scan repo")
  - crates/core/tests/fixtures/agent_summary_turn.jsonl: new fixture
  - crates/cli/src/tui/mod.rs: AgentEnd.task -> summary; agent_done(what); 4 test literals renamed (staged hunks only)
checks:
  - cargo fmt --all -- --check: PASS (exit 0) after wrapping the api.rs import
  - cargo clippy -p chaude-core -p chaude-cli --all-targets -- -D warnings: PASS (exit 0)
  - cargo test -p chaude-core -p chaude-cli: PASS (exit 0; cli 258+4 ok, core lib 243 ok, engine 98 ok, others ok)
  - cargo test -p chaude-core --test engine agent_end_summary: PASS (1 ok)
  - staged-only export cargo check -p chaude-cli --all-targets: PASS (exit 0)
  - cargo build -p chaude-cli --bin chaude: PASS -> target/debug/chaude (includes the parallel worker's uncommitted mod.rs/render.rs edits)
unmatched:
  - parallel worker's crates/cli/src/tui/mod.rs and render.rs edits left unstaged and uncommitted
  - Go lastSummary runs on the capped log (maxAgentLog 400); Rust uses the full stream. Differs only if 400 newer lines are all tool/indent lines.
  - live TUI not run in a TTY; "[agent 완료] ... · <summary>" verified by unit and engine tests only.

## follow-up: stage rows grey + blank row after each (54f8eb1)
task: stage/progress lines grey, separated by one blank row
commit: 54f8eb1 fix(tui): grey stage rows with a blank row after each
changed:
  - crates/cli/src/tui/mod.rs: UiEvent::Stage -> note(GREY_ON + stage_text + FG_OFF) then blank row; test stage_rows_are_grey_and_separated_by_a_blank_row
  - core: not changed (stage text is built in cli only)
affected lines: "· <slot> 처리중|완료|실패: …|번역중|호출" (prompter, main, tui, coder, agent <label>). Not touched: ● Agent, ⎿, [agent 완료], replies, Notice/Error notes.
go parity: deprecated/src/main/intt/internal/harness/tui.go:379-380 note block = dimSt + "" (blank row); resident.go:258 stage(); repl.go:650, 723.
checks:
  - cargo fmt --all -- --check: PASS (exit 0, after rustfmt on mod.rs test)
  - cargo clippy -p chaude-core -p chaude-cli --all-targets -- -D warnings: PASS
  - cargo test -p chaude-cli stage_rows_are_grey_and_separated_by_a_blank_row: PASS
  - cargo test -p chaude-core -p chaude-cli: PASS on rerun (first run: core engine child_pid_recorded_on_park FAILED once, timing flake; passes alone and on rerun)
  - cargo build -p chaude-cli --bin chaude: PASS -> target/debug/chaude
doubts:
  - Go dim = lipgloss faint; Rust uses GREY_ON (136,136,136) per existing note convention. Color choice not verified in a live TTY.
  - record/pastes/2bf311ec.md: the blank-row block is the second (Rust-labelled, "이건 rust") block; the first block has no blanks. Go code itself adds the blank row.
  - child_pid_recorded_on_park flake under parallel load (wait_for timing); not investigated further.
