# tui-parity audit tail (read-only, no build, no test)
workstream: tui-parity audit tail
repo: /Users/leekwangseok/PycharmProjects/claude-harness (branch rust-port, never push)
scope: open [ ] rows E26 E27 E28 X01 X02 X05 X06 X07 X08 X09 X10 X11 X12 X13 A01 A02 (16)
excluded (other coder): C07 E05 E08 E20 E21 E24
counts: audited 16 · same 7 · diff 9
diff ids: E27 X01 X02 X05 X08 X09 X12 X13 A02
go truth: deprecated/src/main/intt/internal/harness/ ; rust: crates/cli/src/tui/, crates/core/src/api.rs
caveat: drain_test.go, bubbletea source, and drain.go internals not read; mouse sequences for Go are from bubbletea convention (not verified here)

## same
- E26: same. go repl.go:964-975 (pending snapshot kept) <-> rust api.rs:2849-2856 start_handoff -> api.rs:3003-3016 rotate_keep_snapshot; live carry go repl.go rotate -> rust api.rs:2935 build_snapshot -> api.rs:3059-3075 rotate_child sets pending_snapshot.
- E28: same. go turn_wait_test.go:38 (open Agent tool_use, no interrupt, late reply kept) <-> rust api.rs:2462-2512 main_wait_budget returns None while open_tools non-empty; api.rs:2547-2549 on_quiet returns early; no cap.
- X06: same. go importclaude.go:196-217 startView before apply <-> rust mod.rs:2443-2444 and 2599-2603 show_import then install_import.
- X07: same. go tui.go:295-311 welcome() at view start (368) <-> rust mod.rs:1790 welcome_rows, shown at startup (mod.rs:3108).
- X10: same (timing differs, output same). go history.go:35-75 load and save (tmp + rename, 0600), saved on each submit via tui.go:272-278 <-> rust edit.rs:200-208 take() saves on submit, edit.rs:301-345 push_history/read_history/write_history (tmp + rename, 0600). Row text says "exit save"; go saves on submit, not exit.
- X11: same. go tui.go:216-217, 1063, quit() 1196-1199 (tea.Quit) <-> rust commands.rs:107-111 names, commands.rs:379 Dispatch::Quit, mod.rs Action::Quit break.
- A01: same. go tui.go:1394 WithAltScreen only <-> rust mod.rs TermGuard::enter (2261 EnterAlternateScreen); no inline viewport path found (grep inline/Viewport in mod.rs; viewport is altscreen-only by design).

## diff (fix specs)
- E27: diff. go repl.go:985-1001 (agents busy and MaxDrain>0: StopIdle, push drain with DrainTimeout, close oldest beyond MaxDrain; else Close) <-> rust api.rs:325-328 max_drain/drain_timeout "No engine effect yet", defaults api.rs:405-406.
  fix spec: crates/core/src/api.rs restart_child (~2978-2990) or rotate_child (3059): if agents busy and cfg.max_drain > 0, do not kill the old child; keep it in a drain list with deadline now+drain_timeout, call stop-idle, evict oldest beyond max_drain (kill); else kill as today. Drain completion removes entry. Test: drain_test.go (not read).
- X01: diff. go repl.go:1364-1368 and 1372-1374 print "chaude: cannot start claude: <err>" and return 1 <-> rust api.rs:3147-3165 start_child emits UiEvent::Error "claude spawn failed" only; TUI keeps running.
  fix spec: crates/core/src/api.rs Engine::start (527-541) await first spawn result via oneshot and return Err; crates/cli/src/tui/mod.rs tui() (2300ff) on Err before TermGuard::enter: eprintln!("chaude: cannot start claude: {e}") and return Err (main.rs maps to exit 1). Plain path (mod.rs plain() ~2595) same message.
- X02: diff. go repl.go:1367 st.Mode()&os.ModeCharDevice != 0 (true for /dev/null) <-> rust mod.rs:2288 io::stdin().is_terminal() (false for /dev/null).
  fix spec: crates/cli/src/tui/mod.rs run() (2283-2292): replace is_terminal with a char-device test on fd 0 (unix: fstat via ManuallyDrop<File>::from_raw_fd(0).metadata()?.file_type().is_char_device(); non-unix keep is_terminal).
- X05: diff. go repl.go:1375-1377 newMetrics header keys: threshold, handoff_validate, max_drain, drain_timeout_s, compact_window, delegation_rule, category_rotate <-> rust api.rs:777 compact_window and metrics header json (~800-804: compact_window, context_window, cache_ttl) — missing threshold, handoff_validate, max_drain, drain_timeout_s, delegation_rule, category_rotate.
  fix spec: crates/core/src/api.rs actor metrics header json (~800-804): add those six keys from cfg (threshold, !no_handoff_validate, max_drain, drain_timeout.as_secs(), !no_delegation_rule, !no_category_rotate); add cfg fields if absent.
- X08: diff. go tui.go:1394 WithMouseCellMotion (cell motion, 1002) <-> rust crates/cli/src/tui/mod.rs:2264 and 2577 EnableMouseCapture; crossterm-0.29.0/src/event.rs:327-329 emits ?1002h and ?1003h (any-motion).
  fix spec: mod.rs TermGuard::enter (2261-2264) and resume at 2577: replace EnableMouseCapture with a custom crossterm Command writing CSI ?1002h (cell motion) and ?1006h (SGR), disable with ?1002l ?1006l; matching DisableMouseCapture replacement in leave path. Bubbletea sequences not read here.
- X09: diff. go repl.go loop/runTUI deferred: saveExitHandoff, markInterrupted, closeDrains, sess.Close, closeAllChildren <-> rust api.rs:934-950 exit handoff write, kill proc/prompter/tui/coders present; no markInterrupted at exit (go agentstore.go:231, linked to E20); no closeDrains (linked to E27).
  fix spec: crates/core/src/api.rs actor Cmd::Shutdown path (~901, 934-950): before kill, call AgentStore interrupt_running (mark running agents interrupted, same as go agentstore.go:231); close drains once E27 lands.
- X12: diff. go tui.go:459 idle-rotate submitted internally, 463 rotate-deferred, 1403-1405 dispatch <-> rust commands.rs:383-416 has model, resume, import, clear, rotate only; api.rs:1012-1022 /clear and /rotate refused when busy (go does not refuse).
  fix spec: crates/cli/src/tui/commands.rs dispatch (383-416): add /idle-rotate and /rotate-deferred mapping to new engine commands (crates/core/src/api.rs submit 1005-1032, dispatch 1034+) that call run_idle_rotation / rotate_deferred; drop the busy refusal for rotate where go queues it (verify go tui.go:459-463 first).
- X13: diff. go tui.go:1408-1412 program error printed "harness: <err>", exit 1 <-> rust crates/cli/src/main.rs:143 eprintln!("chaude: {e}"), exit 1.
  fix spec: crates/cli/src/main.rs:143 change prefix to "harness: {e}" (keep exit 1).
- A02: diff. go repl.go loop 1200-1262 (local commands /quit /exit /rotate /mode /resume /import, localCmd, passthrough; echo "> "+line when !interactive) <-> rust crates/cli/src/tui/mod.rs plain() (~2595-2625): engine.submit per line, no local dispatch, no prompt echo.
  fix spec: mod.rs plain(): route each line through the same commands::dispatch used by tui() (Quit breaks loop, /rotate /mode /resume /import handled locally, passthrough to engine); echo "> " + line when stdin is not a char device (Go !interactive branch).
