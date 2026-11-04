kind: run-record
agent: haiku-tui-a (Rust TUI A coder)
date: 261104
branch: rust-port
scope: crates/cli/src/tui/mod.rs sessview.rs records.rs settings.rs scripts/tui-diff.sh

commits:
  4581714: test(tui): tui-diff scenarios for settings, resume and view
  7699492: fix(tui): grey unselected picker and settings rows like Go dimSt
  1f1cb72: feat(tui): rewind on Esc Esc, agent view, stage notes, overlay wheel

done:
  - rewind wired in mod.rs: Esc Esc on empty input (600 ms, Go tui.go ~709) opens rewind; NOTHING note when no prompt; key routing first while open; cut truncates transcript to prompt row and sets input; REWOUND note; overlay_lines draws rw.lines
  - prompts: Vec<(row, text)> pushed in submit() for echoed prompts (not /view)
  - UiEvent::Stage wired: stage_text() gives Go strings: "· <slot> 처리중" / "· tui 번역중" / "· <slot> 완료" / "· <slot> 실패: <err>"; agent slot "agent <desc>" uses 호출
  - agentview (Ctrl+T) and overlay wheel from earlier steps committed in 1f1cb72
  - clippy -p chaude-cli -- -D warnings: PASS (after core fixed its Shape import error)
  - cargo test -p chaude-cli tui:: : 158 passed, 0 failed
  - scripts/tui-diff.sh (no args = all 14): see table below

tui-diff (match% of 30 rows):
  startup-idle 100, type-text 100, multiline 100, submit-stream 13, tool-use 13,
  slash-menu 100, scroll-up 13, esc-interrupt 66, resize-80x24 12 (24 rows),
  exit 93, usage 96, settings 100, resume 100, view 90

remaining:
  - view: 3 rows (no-sessions note wrap at 100 cells; one extra reset row before input box)
  - submit-stream/tool-use: rust lacks core prompter+tui stage events (core emits main only); no [ctx ...] status line; welcome box position differs
  - no baseline run: low scores on submit-stream, tool-use, scroll-up, resize-80x24 not compared to pre-change numbers
  - rewind path has no unit or tui-diff test (clippy and unit tests only)
  - Task 3: render/bar R03..B17 hooks not wired; #![allow(dead_code)] kept (bar.rs, render.rs are TUI-B)
  - Stage rows are plain grey notes like Go stageMsg; ✻ spinner stays in bar (Go has no ✻ on stage rows)

risks:
  - settings.rs was outside the listed paths; changed for grey rows and Settings::model_id
  - model clamp via Settings::model_id changes welcome model semantics (Go clamp)
  - rewind cut assumes prompts list equals rewind blocks; /view not recorded
  - agent view has no sub-agent logs (UiEvent has none); shows "(no output from this agent yet)"
  - bar.rs and render.rs (TUI-B) remain unstaged and uncommitted
