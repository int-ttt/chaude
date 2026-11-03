# tui-integration-a
branch: rust-port
scope: crates/cli/src/tui/mod.rs full-screen rewrite (alt screen, pure frame(), per-row draw), edit/bar/commands wiring

## done (wired in mod.rs, covered by tests)
- L01 partial: alt screen + viewport + input line + status row (no queue/panel rows yet)
- L02 partial: viewport height recomputed from terminal size each frame (no panel/queue heights yet)
- L04 partial: transcript blocks in order (caveman hiding not wired)
- L12: status row at bottom (bar::rows)
- L14 partial: terminal cursor parked on input line (IME not handled)
- S01: follow tail
- S02: held via wheel, PgUp/PgDn, Ctrl+B/Ctrl+F, Shift+Up/Down
- S03: unseen indicator "↓ 새 메시지" while held (notes/runs/scroll-bottom.md rule)
- S05: Home/End jump when input empty, otherwise edit cursor
- S06: mouse wheel scrolls viewport
- S07 partial: /mouse toggles wheel handling and mouse capture
- editor keys wired (I-IDs done at editor level in notes/runs/tui-input.md): Ctrl+J newline, Alt+Enter newline, trailing-backslash newline, Ctrl+A/E/W/U/K, Up/Down move_row then history, history loaded from <cwd>/.chaude/history.jsonl
- spinner: bar::spinner_frame (Go 125 ms step) drives the running row
- commands::dispatch: /quit /exit (busy-gated), /mouse, Note results; /status /version /mode still go to engine
- Ctrl+C / double Esc interrupt rules kept
- plain() kept for non-tty stdin
- allow(dead_code) removed from edit.rs (bar.rs, commands.rs, render.rs keep it: many helpers still unused)

## partial, not checked
- S04 typed input at bottom re-follows tail: at bottom already follows; typing while held does not change position (spec says held stays)

## remaining
- L03 "starting..." before first size (terminal::size fallback 80x24 used instead)
- L05 welcome banner
- L06 background/sub-agent rows below input
- L07 panel rows above input
- L08 small-height overflow guard for panel/bg/queue
- L09 slash suggestion popup (max 8 rows)
- L10 input grows to 5 rows
- L11 queued lines dimmed above input
- L13 overlay screens (agent, usage, session, settings, rewind, record picker)
- S04 full re-follow on typed input
- S08 mouse drag selection
- S09 selection anchor on content
- S10 copy on release
- S11 clipboard native then OSC 52
- S12 agent view scroll
- S13 usage view scroll
- S14 session view wheel
- S15 record picker wheel/Enter
- S16 panel wheel
- S17 range indicator
- blocked per notes: I14, I16-I18, I25-I33; C08, C10, C11-C16, C29-C33

## checks
- cargo fmt --all -- --check -> PASS
- cargo clippy --workspace --all-targets -- -D warnings -> PASS
- cargo test --workspace -> PASS (144 passed, 0 failed)
- not run: real TTY launch of the binary (alt screen visual check not done)
