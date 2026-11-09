# chaude Rust port plan (decided 2026-11-03)

## Why
- GUI support (Tauri) sharing one core with the terminal UI.
- Safer data handling (serde enums for stream-json events, exhaustive match).
- Drop Charm (Bubble Tea) full-screen TUI; inline output + bottom input line.
- Memory: wrapper 11.2MB (Go) -> 2.2MB (Rust prototype). The 3 Claude processes per turn (prompter, main, haiku-tui) stay concurrent by user decision; no direct Anthropic API calls.

## Fixed constraints
- Binary names chaude / chaude.exe; asset names chaude_<ver>_<os>_<arch>.tar.gz / windows .zip; checksums.txt; targets darwin/arm64, linux/amd64, linux/arm64, windows/amd64. install.sh, install.ps1, `chaude update` keep working unchanged.
- Claude CLI + subscription auth only. Rust style: owned types, no lifetimes in structs, std::thread + mpsc (no async runtime unless GUI needs it), minimal deps.
- Go version on master keeps receiving bug fixes until the switch.
- gen-settings: if the target directory has no settings folder (.claude/ and its hooks/), create it automatically.

## Layout
Cargo workspace at repo root: crates/core (lib), crates/cli (bin chaude), crates/gui (Tauri, later).

## Phases
1. Workspace + CI (cargo build/test on ubuntu/macos/windows) + cmd dispatch, osenv, flag parsing, settings/gen-settings (hooks embedded via include_str!).
2. clean, update, 7 helper scripts (charcount, claude-history, compress-archive, export-history, extract-transcript, search-history, usage-report) with ported tests.
3. Harness logic (~8k Go LOC) into core: stream-json, rotation, handoff, record, orch, resident MCP bridge (mcp-resident, tool resident_coder), import/--from-claude, metrics.
   - status (2026-11-04, crates/core):
     - done: handoff read side, resident and owned session ids (e73f375; handoff::read_exit_handoff, prev_owned).
     - done: resident_coder MCP tool over loopback, core side (f90929d; mcp::start/serve/bridge, Engine coder_caller). `chaude mcp-resident` subcommand wired in crates/cli (mcp_resident.rs, main.rs arm).
     - done: main child idle rotation, Go idleDeadline (f6aca83; Config.idle_rotate).
     - done: resident MCP loopback requires a per-run token line (d50d91f; mcp::new_token 256-bit from /dev/urandom, token in mcp.json env CHAUDE_MCP_TOKEN, not argv).
     - done: record file format (record.rs: render/parse_entries, append_turn with lock, next_number, topic_of, slugify, dir_for; Stamp passed in, no local-time dep).
     - done: mcp temp dir guard (mcp::McpConfig/start_owned removes chaude-mcp-* dir on drop; api.rs uses start_owned, api.rs:2378).
     - done: record_folder.rs (project folders, legacy root migration, list/filter/load, /resume lines; date fn passed in) and record_turn.rs (RecordState topic/save, measure_line, resume, pick).
     - done: orch.rs (Go orch.go, orchws.go: parse/default_model/rule, mode f Router with parse_ws, WsState rotation at 20 turns/40k ctx/3570s idle, handoff_text; api.rs routing hook needed).
     - done: rotate_defer.rs (Go rotate_defer.go: due, Deferral hold/is_due/take with timeout, bg_handoff_lines, running_count; api.rs uses rotate_defer::Deferral, api.rs:2060-2120).
     - done: import/--from-claude core (7b60fb4; import_claude.rs: transcript read, /resume group merge, snapshot, dry-run record; import_view.rs: paired-turn view for the TUI).
     - done: metrics core (cf3d7f1; metrics::usage_fields, metrics_report.rs: summary text and 14-column CSV from the per-day JSONL files).
     - done: api.rs turn-end RecordState::save (api.rs:1996), crates/cli call sites for /import --from-claude, --import-dry-run and `chaude metrics` (--dir, --csv).
     - open: api.rs wiring of orch Router (orch.rs is not called from api.rs or cli, verified 2026-11-04 at c8233d6).
     - wired in api.rs (2026-11-04, a9c1d6d and earlier): usage poll after each turn (5 min throttle), turn spend merged into the turn metrics row, delegation rule in the main child system prompt, /clear clear mark, client registry (clients.json: live row on child start, clear ends it, startup reconcile emits OrphanNotice), AgentStore startup interrupt sweep.
     - open (not wired in api.rs, not done): haiku_pool WorkerPool note_edit (Go repl.go:743), pick_child with ChildSet (Go dispatch.go:91, needs multi-child routing), orch Router route/record, AgentTracker persistence per agent (agent_store.rs AgentTracker not used by api.rs).
4. Terminal UI without Charm: inline streaming output, input line with Korean IME + unicode width, spinner, status/usage bar, slash commands.
   - status (2026-11-04, crates/cli/src/tui):
     - done: inline streaming output (live feed + record load, asst rows via markdown path, tool_result_text).
     - done: input line Korean IME + unicode width (render::input_view_row, cursor cell).
     - done: spinner (bar::spinner_frame).
     - done: slash commands in the TUI (/usage /settings /view /mouse /update, record /resume). /status /version /mode and /clear go to the engine.
     - done: idle "still waiting" notice (Go quiet, 30 min, no open tool call).
     - done: /view lists the previous run's owned sessions (handoff::read_exit_handoff, prev_owned).
     - done (f73b6fb): status bar P/T/C per-role context (UiEvent::RoleContext), /view this run's session ids (UiEvent::SessionIds), /clear new-session notice (UiEvent::NewSession), thinking row, rotated notice, orphan notice, tool summary.
     - done: done row (K8, bar::done_line, UiEvent::AgentEnd) and assistant link wrap (K12, render::wrap_linked in body()). Checklist rows K8 and K12 are done (verified 2026-11-04).
5. GUI (Tauri): session tabs, conversation view, input, usage display; .dmg / .msi.
   - status (2026-11-04, crates/gui): placeholder only.
     - done: crates/gui (chaude-gui) is a workspace member again (exclude removed); depends on chaude-core by path only, no UI framework.
     - done: event_kind wraps core event::parse_line + type_name (unit tests: unknown type, invalid JSON).
     - open: Tauri dependency and UI (session tabs, conversation view, input, usage display, .dmg / .msi).
6. Parity tests vs Go (320 Go tests as reference), switch release to Rust, keep Go one version in deprecated/.
- Priority (2026-11-03): Mac first; engine + TUI before phase 2 tools. See plan/rust-engine-tui.md.

## Open
- Which Charm-era features to keep: agent view, session list view, mouse selection/clipboard.
