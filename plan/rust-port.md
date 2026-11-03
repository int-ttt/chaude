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
4. Terminal UI without Charm: inline streaming output, input line with Korean IME + unicode width, spinner, status/usage bar, slash commands.
5. GUI (Tauri): session tabs, conversation view, input, usage display; .dmg / .msi.
6. Parity tests vs Go (312 Go tests as reference), switch release to Rust, keep Go one version in deprecated/.
- Priority (2026-11-03): Mac first; engine + TUI before phase 2 tools. See plan/rust-engine-tui.md.

## Open
- Which Charm-era features to keep: agent view, session list view, mouse selection/clipboard.
