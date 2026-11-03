# Rust engine + TUI design (Mac first, 2026-11-03)

## Scope
macOS (darwin/arm64) first. No Windows-specific code yet; keep code portable (crossterm). Order: E1 -> T1 -> E2 -> T2 -> E3.

## Engine (crates/core)
- event.rs: serde enums for claude stream-json output (system, stream_event, assistant, user, result, control_request, rate_limit_event, Unknown). Promote from deprecated/prototype/chaude-rs.
- proc.rs ClaudeProc: spawn `claude` with the exact args the Go harness uses; reader thread -> mpsc; writer sends user messages and control messages (interrupt, can_use_tool replies).
- api.rs (the only surface frontends see):
  - Engine::start(Config) -> (EngineHandle, Receiver<UiEvent>)
  - EngineHandle: submit(String), interrupt(InterruptSource), slash(String), shutdown()
  - UiEvent: TextDelta{role,text}, MessageDone{role}, ToolStart{id,name,summary}, ToolEnd{id,ok}, AgentStart{id,description,started_at}, AgentEnd{id,elapsed_ms}, Status{model,tokens,cost_usd,context_pct}, Busy(bool), Queued(usize), TurnEnd{duration_ms}, Rotated{reason}, Error(String)
  - InterruptSource: EscKey, CtrlC, Signal, Api — every interrupt is logged with its source (fixes the unexplained 16:40 interrupt class of bug).
- Input queue lives in the engine: submit while busy only queues and emits Queued(n); it never interrupts.
- pipeline.rs (E2): per user input, prompter, main and haiku-tui Claude processes run concurrently, exactly as the Go orch does. record + metrics writers.
- E3: rotation/handoff, import / --from-claude, resident MCP bridge (mcp-resident, resident_coder).

## TUI (crates/cli), no Charm, inline
- Finished output lines are printed into the normal terminal scrollback (terminal owns history; chaude memory stays flat). No alternate screen.
- Bottom region redrawn in place (3 lines): running agents with elapsed time + spinner, status bar (model, tokens, cost, context %, queued count), input line.
- Input editor: UTF-8, Korean via terminal IME (committed chars), unicode-width cursor math, history (Up/Down), Shift+Enter or Ctrl+J newline.
- Keys: Enter submit (queues if busy); Esc twice within 1s = interrupt (single Esc never interrupts); Ctrl+C = interrupt when busy, exit when idle with empty input.
- T2: slash commands (/help /status /model /usage /cost /context /update /exit ...), light markdown to ANSI (headers, bold, code, lists, tables), usage bar.
- Deps: crossterm, unicode-width, serde, serde_json.
