# LESSONS
- 2026-10-29 failed: Hangul "한칸 밀림" fixed by adding bgH to curUp (0.6.11) -> no effect. Shift is a column problem: JediTerm draws IME preedit at cursorX+1. Check terminal source (JediTerm TerminalPanel) before tuning cursor rows.
- 2026-10-29 failed: /resume fix rejected earlier because session was draining; never applied. Import must also be shown in TUI blocks, and rotate must not handoff/overwrite a pending import snapshot.
