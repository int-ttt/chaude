# GUI parity plan (2026-11-12)

Goal: `chaude-gui` desktop app with every TUI feature, released in 0.13.0. User order: "gui에서 tui 모든 기능 전부 구현하고, 0.13.0으로 릴리즈해".

## Decisions
- Framework: eframe/egui (replaces Tauri 2 from plan/archive/gui-basic.md). Pure Rust, one binary, no JS.
- Binary: `chaude-gui` in crates/gui (add `[[bin]]`), depends on chaude-core only. Add crates/gui to workspace default-members.
- TUI behaviour must stay identical. Shared logic moves to core; TUI then calls core.
- Markdown: egui_commonmark. Notifications: notify-rust (completion notice, GUI only per notes/OPEN.md).
- Linux build deps for CI: libxkbcommon-dev libgl1-mesa-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libwayland-dev.

## [haiku] steps
G0 core extraction (one coder)
- Move slash-command parse + dispatch enum from crates/cli/src/tui/commands.rs to crates/core (TUI re-uses it).
- Move doctor checks (crates/cli/src/doctor.rs checks_now) to core; cli keeps a thin wrapper.
- Move settings retention/extra_coders fields from tui/settings.rs to core chaude_settings.rs.
- Derive serde::Serialize on UiEvent if useful; otherwise skip.
- Gate: all existing tests pass, TUI unchanged.

G1 GUI skeleton
- eframe app; start the main session through the same core api the TUI uses.
- Output pane: streamed events, markdown, links, auto-follow tail when at bottom, keeps scroll otherwise.
- Input box: multi-line, Enter sends, history from .chaude/history.jsonl.
- Status bar: M/P/T/C context rows + turn token line (same text as tui/bar.rs, via core i18n).
- Tasks panel: rows with status, title, `Nk 토큰 · Ns`; restored from notes/tmp/long/tasks-<sid>.json.

G2 commands and keys
- All slash commands from the TUI list (/lang, /doctor, /resume, /update, /mode, /usage, /import, /handoff, /exit, /clear, settings…), with a command palette/autocomplete.
- Esc cancels wait; Esc Esc rewind; Ctrl+T agent view window; Ctrl+C/Ctrl+D same as TUI.
- Permission prompt as a y/n dialog. agent_wait queue (hold/release/cancel via core).
- Settings window (model, agent_wait, retention, extra coders, language).
- Notices (orphan, budget, rotated, quiet) as banners; OS notification on turn complete.

G3 parity check
- Table in this file: every TUI feature | GUI where | verified how. Fill gaps.
- All user-visible strings through core i18n (ko/en).

G4 release 0.13.0
- release.yml: build chaude-gui for darwin-arm64, linux-amd64, linux-arm64, windows-amd64; include in archives.
- README.md + README.ko.md: GUI section (launch, features). CHANGELOG en+ko.
- Bump 0.13.0 like commit a37b862, tag v0.13.0, push, watch CI, mac install (chflags nouchg/uchg).

## Rules per step
- Gate via haiku-test-lead: fmt, clippy -D warnings (mac + windows target), tests core/cli/gui.
- Commit only that step's files: `gui: <what>` + CHANGELOG Unreleased en+ko line. Push only at G4.
- rm -rf target/debug after use. Scratch only in notes/tmp/test/.

## G3 parity table (G0, 2026-11-14)
Basis: crates/cli/src/tui/*.rs, crates/core/src/slash.rs (SLASH_CMDS, PASSTHROUGH, UNSUPPORTED), crates/core/src/api.rs (EngineHandle, UiEvent). "core ready?" is judged by inspection, not by a test. Size is a judgment from TUI code volume (mod.rs alone is 8502 lines): S = under ~150 lines, M = 150-600, L = over 600. GUI status is "not started" for all rows until G1/G2 land.

### TUI features
| feature | TUI source | core ready? | GUI status | size | phase |
|---|---|---|---|---|---|
| Output stream (text, tool blocks, markdown) | tui/render.rs, tui/records.rs, tui/sessview.rs | yes (event.rs, UiEvent) | partial: engine wiring and reducer done (G1 commit 1); pane and markdown in G1 commit 2 | L | G1 |
| Multi-line input, Enter sends, history | tui/mod.rs (key handling ~1085-1290) | partial (history file path needs check) | not started | M | G1 |
| Status bar M/P/T/C rows and turn token line | tui/bar.rs | partial: orphan/budget/rotated/quiet notices hard-coded English (bar.rs:381-406), not i18n | not started | M | G1 |
| Tasks panel (status, title, Nk tokens, Ns) | tui/mod.rs (UiEvent::TaskStats ~630) | yes | not started | M | G1 |
| Edit and clipboard | tui/edit.rs, tui/mod.rs (~2927-3024) | n/a (toolkit side) | not started | M | G1 |
| Agent view window (Ctrl+T) | tui/agentview.rs | yes (agent_rows) | not started | M | G2 |
| Session view | tui/sessview.rs | yes (UiEvent stream) | not started | M | G2 |
| Permission prompt (y/n) | tui/mod.rs (~968) | yes (answer_permission) | not started | S | G2 |
| agent_wait queue hold/release/cancel | tui/mod.rs | yes (cancel_wait) | not started | S | G2 |
| Esc cancels wait, Esc Esc rewind | tui/mod.rs, tui/rewind.rs | partial (interrupt yes; rewind flow in TUI only) | not started | M | G2 |
| Ctrl+C / Ctrl+D | tui/mod.rs | yes (interrupt) | not started | S | G2 |
| Resume picker | tui/resume_pick.rs | yes (resume_session) | not started | M | G2 |
| Settings screen (model, agent_wait, retention, extra coders, language) | tui/settings.rs | partial (retention/extra_coders fields in core per G0 plan) | not started | M | G2 |
| Language switch | tui/lang.rs | yes (core i18n.rs) | not started | S | G2 |
| Usage overlay | tui/usage.rs | yes (usage) | not started | S | G2 |
| Help and popup overlay | tui/overlay.rs | n/a (UI only) | not started | S | G2 |
| Keyburst effect | tui/keyburst.rs | n/a (TUI-only effect; keep or drop in G2) | not started | S | G2 |
| Notices: orphan, budget, rotated, quiet | tui/bar.rs, tui/mod.rs (~697, ~814, ~931) | partial (English text, see bar.rs row) | not started | S | G2 |
| OS notification on turn complete | (new, GUI only per notes/OPEN.md) | yes (notify-rust dep, 101a788) | not started | S | G2 |

### Slash commands (crates/core/src/slash.rs)
| feature | TUI source | core ready? | GUI status | size | phase |
|---|---|---|---|---|---|
| /help | slash.rs SLASH_CMDS | yes (HELP_KEYS) | not started | S | G2 |
| /clear | slash.rs | yes (dispatch) | not started | S | G2 |
| /status | slash.rs | yes | not started | S | G2 |
| /model | slash.rs | yes | not started | S | G2 |
| /settings | slash.rs | partial (see Settings row) | not started | M | G2 |
| /lang | slash.rs | yes (i18n) | not started | S | G2 |
| /resume | slash.rs | yes (resume_session) | not started | M | G2 |
| /import | slash.rs | yes (install_import) | not started | M | G2 |
| /view | slash.rs | yes | not started | S | G2 |
| /rotate | slash.rs | yes (rotate.rs) | not started | S | G2 |
| /usage | slash.rs, BUSY_OK | yes (usage) | not started | S | G2 |
| /cost | slash.rs PASSTHROUGH | yes (passthrough to Claude CLI) | not started | S | G2 |
| /context | slash.rs PASSTHROUGH | yes (passthrough) | not started | S | G2 |
| /compact | slash.rs PASSTHROUGH | yes (passthrough) | not started | S | G2 |
| /config | slash.rs PASSTHROUGH | yes (passthrough) | not started | S | G2 |
| /mcp | slash.rs PASSTHROUGH | yes (passthrough) | not started | S | G2 |
| /agents | slash.rs PASSTHROUGH | yes (passthrough) | not started | S | G2 |
| /init | slash.rs PASSTHROUGH | yes (passthrough) | not started | S | G2 |
| /review | slash.rs PASSTHROUGH | yes (passthrough) | not started | S | G2 |
| /doctor | slash.rs, doctor.rs | yes (doctor::checks_now) | not started | S | G2 |
| /mode | slash.rs | yes (set_mode) | not started | S | G2 |
| /mouse | slash.rs | n/a (terminal mouse capture, TUI-only) | not started | S | G2 (mark N/A) |
| /version | slash.rs | yes | not started | S | G2 |
| /update | slash.rs | partial (check core update path) | not started | M | G2 |
| /exit, /quit | slash.rs | yes (app exit) | not started | S | G2 |
| /memory | slash.rs UNSUPPORTED | no (unsupported in TUI too) | not planned | S | none |
| /permissions | slash.rs UNSUPPORTED | no (unsupported in TUI too) | not planned | S | none |

Gaps to fill in G1/G2: (1) bar.rs notice strings go through core i18n (ko/en); (2) confirm history file, update path, and settings fields in core before marking "yes".
