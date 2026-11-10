# chaude GUI — basic design (2026-11-10)

## Goal
A desktop window for chaude with the same features as the TUI: main conversation, agent list + agent log, input, status line. Same core, same sessions, same config. Mac + Windows first.

## Tech choice: Tauri 2 (Rust backend + web view)
- Runs chaude-core in-process (no IPC to a CLI).
- Korean IME works natively in a web view; egui/iced IME support is weak.
- Web view gives selection, copy, links, markdown/code rendering for free.
- Mac uses WKWebView, Windows uses WebView2 (preinstalled on Win10/11). App ~10 MB.
- Frontend: Svelte + Vite + TypeScript (small bundle, simple reactivity).

## Architecture
1. View-model in core: move the TUI state reducer (agent rows, status/model/type/start/duration, log lines with arrival time, tool result placed under its tool_use_id, coder slots C1..Cn, notices) into `chaude-core::view`. TUI and GUI both consume it, so behavior stays identical.
2. Events: UiEvent / view updates derive `serde::Serialize`. GUI backend forwards them to the frontend as Tauri event `view-update` (JSON patch or full row/line upserts keyed by id).
3. Commands (frontend → backend): `send_prompt(text)`, `interrupt()`, `select_agent(id)`, `resume_agent(id)`, `open_path(path)`, `open_url(url)`, `copy(text)`, `check_update()`.
4. Crate `crates/gui` (Tauri app) produces binary `chaude-gui`; shares config, session store, agent store, update crate with the CLI.

## Layout (basic)
- Top bar: project path, git branch, chaude version, update badge.
- Left: agent list — `HH:MM:SS status · type · model (task) (duration)`, live ticking duration, status colors (running/done/failed/stopped).
- Center: main conversation — user/assistant messages (markdown), tool calls green `● HH:MM:SS Bash(...)`, results `⎿` collapsed to first line with expand, timestamps, links/paths clickable.
- Right drawer: selected agent log (same line format), opens on click in agent list, closable.
- Bottom: multiline input (Enter send, Shift+Enter newline, Esc interrupt, Up = history); under it the status line: main model, context used/limit, `C1 x/80k C2 x/80k`, idle timer, rotation marker.
- Scroll: follow tail only when at bottom; "↓ new messages" button otherwise.

## Phases
- G0: extract view-model to core; TUI switched to it; all existing tests pass. Add serde to events.
- G1: Tauri shell: window, input, streaming main pane, interrupt. Accept: one prompt round-trip, Korean input, Esc interrupt.
- G2: agent list + agent drawer + status line with coder slots. Accept: background agent shows running→done with ticking duration; restored rows after restart.
- G3: packaging + release: mac .app/.dmg (aarch64), Windows .exe/.msi; release workflow adds assets; README/README.ko section; /update covers chaude-gui.

## Out of scope (basic)
Multiple session tabs, settings UI, themes, diff viewer, OS notifications, Linux packaging.

## Open decisions
- Mac code signing/notarization (Apple Developer $99/yr); unsigned app shows a Gatekeeper warning.
- One binary with `chaude gui` subcommand vs separate `chaude-gui` (plan: separate; avoids web view deps in the CLI).
