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
