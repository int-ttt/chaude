# tui-parity final-1 run note
workstream: tui-parity final-1
branch: rust-port
truth: deprecated/src/main/intt/internal/harness/

batch-1 (rows audited: 15)
rows-same: I27 I28 I29 I30 I31 I32 I33 I34 I35 C01 C02 C03 C04 C05 C26
rows-fixed: none in this batch
rows-diff: none in this batch
C01-check: normalized name|desc compare of Go slashCmds (25) vs Rust SLASH_CMDS (25) -> identical
C02-check: Go suggestions tui.go:982-994 vs Rust suggestions commands.rs:344 -> same guards (/ prefix, no space/tab, not busy, no perm)
C03-check: help text = "%-8s desc" rows + HELP_KEYS, identical to Go tui.go:1057-1061
C04-check: /exit,/quit -> Quit, same as Go quit() tui.go:1196
C05-check: toggle + mouse_note strings identical; capture enable/disable synced in run loop mod.rs:2572
C26-check: BUSY_OK list identical to Go busyOK tui.go:849 (includes /settings, which the plan list omits); busy message identical
remaining-first-half: C07 C08 C09 C14 C15 E01 E02 E03 E04 E05 E06 E07 E08

batch-2 (rows audited: 13)
rows-same: C08 C09 C14 C15 E01 E02 E03 E04 E06 E07
rows-fixed: none in this batch
rows-diff: C07 E05 E08
C08-check: RETAIN/MODEL/extra-coder choices equal settings_picker.go:17-18, chaude_settings.go:18; Rust settings.rs:16-23; save on esc/q settings.rs:177-188
C09-check: save message and empty-path message identical to settings_picker.go:53-55 (settings.rs:190-193)
C14-check: pure part only: list_draws_sessions_and_esc_closes (sessview.rs:621) vs TestViewEscBacksOutToNormalScreen (sessview_test.go:35); the model-level esc test is not ported
C15-check: ACTIVE_WINDOW 10 min = sessActiveWindow; "● " marker sessview.rs:408 = sessview.go:218-219
C07-diff: TUI refuses /model NAME (mod.rs:1007-1011); core lacks rotate-with-model (needs park old child, set model, handoff-rotate without held prompt)
E01-check: TextDelta/MessageDone apply mod.rs:311-337; fake-stream equivalent engine.rs:211 (Go acceptance test is live-only)
E02-check: on_tool_use id dedupe api.rs:2164-2167 = Go seenTools; ToolStart row mod.rs:347
E03-check: caveman return hidden mod.rs:382-399; test mod.rs:3296 = TestToolResultCavemanHidden
E04-check: Agent/Task delegations api.rs:2195-2200; slow-agent wait test engine.rs:601 = TestTurnWaitsForOpenAgentToolResult
E05-diff: agentview.rs sub_event_lines (118) and push_log cap 400 (171) have no production caller; api.rs:2007 drops subagent events
E06-check: mode::decide before prompt api.rs:2091; ask path mod.rs:571; engine.rs:465
E07-check: y/Y/Enter allow, n/N/Esc deny mod.rs:648-650 = tui.go:598-607; test mod.rs:3729
E08-diff: plain no-UI idle denial note not ported; TUI path unreachable in Go too (repl.go:574-600); api.rs:2097 comment
remaining-from-batch-1-list: none (C07 C08 C09 C14 C15 E01-E08 all judged; C07 E05 E08 left as diff)
acceptance: cargo test --workspace (HOME=$(mktemp -d)) -> 639 passed, 0 failed, 64 ignored; clippy --workspace --all-targets -> 0 warnings; cargo fmt --all --check -> exit 0
build-note: first workspace runs hit stale chaude-core artifacts (E0599 UiEvent::Notice in engine.rs:1525/1574 while -p build passed); cargo clean -p chaude-core fixed it, no source change
