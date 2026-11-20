# HTML GUI (replaces egui GUI; ships in 0.13.0)

Decision (user, 2026-11-15): option B. 0.13.0 waits for the HTML GUI. egui GUI is retired, never released. Reason: egui GUI opened 4 programs, hard to use, no gain over TUI.

## Shape
- One process, one native window: tao + wry webview in crates/gui (bin chaude-gui). No egui. Same HTML can later be served on localhost (not in 0.13.0).
- Frontend: plain HTML/CSS/JS (no Node build step), embedded with include_str!; markdown via a small vendored lib (e.g. marked + highlight.js, MIT), sanitized.
- Bridge: Rust -> JS events as JSON (RunState/Transcript diffs: entry_push, entry_update, entry_cut, state, picker_open/close, note); JS -> Rust commands via IPC (submit, interrupt, slash, picker_pick, permission_answer, copy). All logic stays in core (ui_state, slash dispatch, rewind, record_pick, session_view, usage/lang/settings models).
- Look: claude.ai-style layout (left session sidebar, centered chat column, rounded composer at bottom, quiet colors, light/dark). Own name/icon; no Anthropic logo, fonts, images or CSS; open fonts only.

## Beyond TUI (why a GUI)
- Sidebar: sessions/records list with search; click to resume/view/import (replaces /resume /view /import pickers).
- Messages: rendered markdown, code blocks with copy button, tables, collapsible tool calls + results, agent rows as a side panel with live status.
- Composer: multi-line, IME-safe (webview), Enter send / Shift+Enter newline, slash menu, drag-drop files and pasted images as attachments (if engine supports; else later).
- Settings, usage, language as modal pages; rewind via hover "edit from here" on prior prompts.
- Native notification on turn end; status (model, context, rate) in a slim header.

## Two modes (user, 2026-11-16)
- Start screen: "New chat" or "Open folder" (+ recent folders). Mode shown in header; each sidebar item remembers its mode.
- Chat mode (like the Claude app): runs the same claude login in a private scratch dir (~/.chaude/chat/<id>), file/shell tools off, no harness agents; plain conversation, markdown, code copy, attachments if supported. History in the sidebar.
- Code mode (like an IDE): native "Open folder" dialog (rfd); engine runs with cwd = folder (full chaude harness). Layout: left file tree of the folder (lazy, .gitignore-aware, live refresh), center chat, right panel tabs: file viewer (read-only, syntax highlight) and diffs of files changed this turn; click a tool row or file to open it; "Open in editor" button (system default / $EDITOR). No in-app editing in 0.13.0.

## [haiku] steps
1. Skeleton: tao+wry window in crates/gui, embedded index.html, IPC ping both ways, single-instance guard; remove egui deps/code behind it. Tests: bridge serialization unit tests.
2. Event protocol in core: serde types for events/commands from RunState/Transcript; snapshot tests of JSON.
3. Chat view: transcript render (text, tool, result, notes, stage), streaming updates, composer submit/interrupt, permission prompt.
4. Start screen + modes: New chat / Open folder (rfd) / recent folders; chat-mode engine config (scratch dir, tools off, no agents); header with mode, model, context, rate; turn-end notification.
5. Code mode IDE layout: file tree (core lister, gitignore-aware, notify watcher), file viewer, per-turn diff panel, open-in-editor; agent side panel + queue/task rows.
6. Sidebar + pickers: [sidebar part moved earlier, done in 261116 sidebar: New chat, Open folder, recent folders, chat scratch-dir list; chat click is TODO until core can resume] chats and folders history, resume/view/import, rewind "edit from here", slash menu, settings/usage/lang modals.
7. Polish: light/dark, shortcuts, attachments (if engine supports), fix focus-3x guard log.
Each step: fresh haiku-coder, cargo test --workspace PASS, clippy --workspace --all-targets + fmt clean, CHANGELOG Unreleased en+ko, local commits, no push. Live smoke: launch window, submit prompt, see reply.

## Observed: 4 GUI processes
- .run/*.xml: none is a compound (no <list>/CompoundRunConfiguration); each is a single cargo run, so none starts 4 at once.
- crates/gui/src/app.rs:244,253: std::thread::spawn (doctor/update jobs), threads not processes.
- crates/gui/src/engine.rs:27: std::thread::spawn (core event pump), thread not process.
- crates/gui: no ViewportBuilder/show_viewport or process::Command; only app.rs:205 ViewportCommand::Close.
- Likely cause: 4 separate runs or chaude-gui launched 4 times by hand, not by config; not confirmed.
