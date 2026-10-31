# sonnet direct read diagnosis (read-only)
cause_confirmed:
- CLAUDE.md:10 forbids sonnet-analyst direct file access, but .claude/agents/sonnet-analyst.md:7 `omitClaudeMd: true` -> agent never sees CLAUDE.md rule (sonnet-reviewer.md:7 same)
- sonnet-analyst.md:4 tools include Read, Grep, Glob, Write, Edit, Bash (no restriction)
- sonnet-analyst.md:18 instructs "Scope first: read at most 3 relevant files (partial reads)" -> directly contradicts CLAUDE.md:10 (max 3 files/bundles via Haiku)
- sonnet-analyst.md:19-20: Haiku delegation is "when they can"/"mechanical sub-steps"/"default for code changes" only; analysis reading is not routed to haiku
- settings.json:61-71 haiku-context-guard.py matches Read; .claude/hooks/haiku-context-guard.py:22 explicitly skips agent_type starting with "sonnet-"/"opus-" -> no enforcement for sonnet
- main-thread-guard.py:20 allows any call with agent_id (subagents) -> no block
- settings.json:6 CLAUDE_CODE_SUBAGENT_MODEL=haiku irrelevant (agent frontmatter model: sonnet)
inference: CLAUDE.md:10 is a main-thread-only instruction that never reaches the agent; no hook enforces it
not_checked: runtime transcripts (whether actual Reads occurred); that omitClaudeMd fully drops CLAUDE.md (per name/semantics)
fix_options: (a) rewrite sonnet-analyst.md:18 to "do not Read files directly; bundle <=3 and delegate to haiku-scout, take summary" + (b) optionally remove Read/Grep/Glob from tools:4 (but it must still Read briefed paths/write notes) or add deny in haiku-context-guard.py:22 for sonnet-analyst
