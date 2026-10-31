# LESSONS
- 2026-10-29 failed: Hangul "한칸 밀림" fixed by adding bgH to curUp (0.6.11) -> no effect. Shift is a column problem: JediTerm draws IME preedit at cursorX+1. Check terminal source (JediTerm TerminalPanel) before tuning cursor rows.
- 2026-10-29 failed: /resume fix rejected earlier because session was draining; never applied. Import must also be shown in TUI blocks, and rotate must not handoff/overwrite a pending import snapshot.
- [measure] 261031: claude spawned inside a Claude session inherits CLAUDE_CODE_CHILD_SESSION -> "Transcript saving is off", no JSONL. Pop CLAUDECODE/CLAUDE_CODE_CHILD_SESSION/SESSION_ID/SESSION_ATTENDED/ENTRYPOINT/MESSAGING_*/CLAUDE_PID in child env; keep config CLAUDE_CODE_* vars. Trust dialog default "No, exit": send Down+Enter.
