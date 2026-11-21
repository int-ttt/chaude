# GUI parity for 0.13.0 (rewritten 2026-11-15)

State: GUI input uses core Editor (crates/gui/src/input.rs); TUI and GUI share core RunState and Transcript (crates/core/src/ui_state.rs). Paths: tui=crates/cli/src/tui/, gui=crates/gui/src/.

## [haiku] steps
1. Transcript rows (S): tool start/end stamps + ToolInput summary update (tui/mod.rs:370-434); tool result rows "  ⎿  head" pushed (434-455); stage rows Working/Done (632-669); wide output (191-193, 1946-1950); rotation notices NewSession/RotatedFrom/Rotated (597-619); orphan notice (604-614); budget stop (723-727); idle notice after 30 min (80, 800-812); "[interrupted]" (695-703); mode row (706-711); welcome banner (2710-2748).
2. Keys (S): Esc cancel wait/interrupt (1032-1034); Ctrl+C interrupt-or-quit, Ctrl+D quit (979-988); Emacs keys + Alt+Enter/Alt+Backspace (989-1070); PageUp/Down/Home/End scroll (1104-1120); permission Enter allow / Esc deny / Ctrl+C deny+interrupt + echo (833-858).
3. Bar and panels (M): M/P/T/C bar, spinner, rate window (tui/bar.rs:41-68, core/bar_text.rs:174-204, RateLimit in gui/state.rs); agent segments; coder context line (bar.rs:69-72); queue rows (2111-2137); task panel (2138-2203); fg/bg agent rows (2204-2213); busy row (1957-1973); scroll range + unseen-output bar; rate-limit text (719-722); GUI completion notification (notify-rust declared, no call site).
4. Slash commands (M): move local slash dispatch (tui/mod.rs:1162-1300, core/slash.rs) into core; GUI runs /help /status /settings /mode /doctor /usage /update /version /exit locally; slash popup + Tab complete (tui/overlay.rs); language picker (tui/lang.rs); usage overlay (tui/usage.rs).
5. Pickers (L): Esc Esc rewind (tui/rewind.rs, 912-935, 1035-1058); settings picker (tui/settings.rs); record picker (tui/records.rs); /resume group picker (tui/resume_pick.rs); /view browser (tui/sessview.rs); /import (1271-1317); mid-run resume list (685-694); session ids in /status (575-596).
6. Verify (S): copy/select + clipboard (2355-2470); md tables in marked (GFM); paste-burst Enter (tui/keyburst.rs). N/A: /mouse.

Each group: fresh haiku-coder, shared logic in core, cargo test core/cli/gui PASS, clippy --workspace --all-targets + fmt clean, CHANGELOG Unreleased en+ko line, local commits, no push.
