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
     - done: resident_coder MCP tool over loopback, core side (f90929d; mcp::start/serve/bridge, Engine coder_caller). Gap: `chaude mcp-resident` subcommand missing in crates/cli.
     - done: main child idle rotation, Go idleDeadline (f6aca83; Config.idle_rotate).
     - done: resident MCP loopback requires a per-run token line (d50d91f; mcp::new_token 256-bit from /dev/urandom, token in mcp.json env CHAUDE_MCP_TOKEN, not argv).
     - done: record file format (record.rs: render/parse_entries, append_turn with lock, next_number, topic_of, slugify, dir_for; Stamp passed in, no local-time dep).
     - done: mcp temp dir guard (mcp::McpConfig/start_owned removes chaude-mcp-* dir on drop; api.rs still calls mcp::start, hook needed).
     - done: record_folder.rs (project folders, legacy root migration, list/filter/load, /resume lines; date fn passed in) and record_turn.rs (RecordState topic/save, measure_line, resume, pick).
     - open: api.rs turn-end call into RecordState::save and /resume wiring (A-owned hook), orch, rotation details beyond idle, import/--from-claude, metrics.
4. Terminal UI without Charm: inline streaming output, input line with Korean IME + unicode width, spinner, status/usage bar, slash commands.
   - status (2026-11-04, crates/cli/src/tui):
     - done: inline streaming output (live feed + record load, asst rows via markdown path, tool_result_text).
     - done: input line Korean IME + unicode width (render::input_view_row, cursor cell).
     - done: spinner (bar::spinner_frame).
     - done: slash commands in the TUI (/usage /settings /view /mouse /update, record /resume). /status /version /mode and /clear go to the engine.
     - done: idle "still waiting" notice (Go quiet, 30 min, no open tool call).
     - done: /view lists the previous run's owned sessions (handoff::read_exit_handoff, prev_owned).
     - blocked: status bar P/T/C per-role context (hardcoded 0/80k); UiEvent has no per-role context.
     - blocked: /view this run's session ids (main, rotated, resident); no UiEvent carries them.
     - blocked: /clear new-session notice; engine does not emit a new-session event.
     - blocked: thinking row, done row, rotated notice, orphan notice, tool summary, assistant link wrap; each needs core data (see targeted #[allow(dead_code)] reasons in bar.rs and render.rs).
5. GUI (Tauri): session tabs, conversation view, input, usage display; .dmg / .msi.
6. Parity tests vs Go (312 Go tests as reference), switch release to Rust, keep Go one version in deprecated/.
- Priority (2026-11-03): Mac first; engine + TUI before phase 2 tools. See plan/rust-engine-tui.md.

## Open
- Which Charm-era features to keep: agent view, session list view, mouse selection/clipboard.
