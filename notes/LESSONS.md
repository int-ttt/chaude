# LESSONS
- 2026-10-29 failed: /resume fix rejected earlier because session was draining; never applied. Import must also be shown in TUI blocks, and rotate must not handoff/overwrite a pending import snapshot.
- [measure] 261031: claude spawned inside a Claude session inherits CLAUDE_CODE_CHILD_SESSION -> "Transcript saving is off", no JSONL. Pop CLAUDECODE/CLAUDE_CODE_CHILD_SESSION/SESSION_ID/SESSION_ATTENDED/ENTRYPOINT/MESSAGING_*/CLAUDE_PID in child env; keep config CLAUDE_CODE_* vars. Trust dialog default "No, exit": send Down+Enter.
- 261105 temp HOME without pinned CARGO_HOME/RUSTUP_HOME re-downloads the registry and fills the disk; scripts/test-isolated.sh pins them.
