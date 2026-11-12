# Subagent concurrency and usage

> Moved 2026-11-12 from plan/caveman-structure.md section 16 (decision history; not a current design).

## 16. Session count: prompter and haiku-tui

- Both are Haiku subagent sessions. All agents (any model) and all surfaces (claude.ai, Claude Code, Desktop, IDE) draw from one usage pool (support.claude.com/en/articles/11647753). Documented concurrency cap = subagents only, default 20 (CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS, code.claude.com/docs/en/sub-agents).
- Usage notice "31% · Run fewer sessions at once · up to 8 at once": 31% = 5-hour window used (weekly <30% on 2026-11-04). "Run fewer sessions" = /usage tip, not a cap. "8" meaning undocumented (likely observed peak; unverified).
- prompter: runs once per user message, before Opus main receives it; summarizes intent. Finishes before main delegates → never overlaps coders. Cost: one short Haiku call per message.
- haiku-tui: runs once after main's reply is final; renders Korean 존댓말. Never overlaps coders. Main replies in Korean directly → no tui session.
- Peak concurrent sessions = main + agents delegated in one round. prompter/tui add 0 to peak.
- Usage driver = coders (long context, many tool calls), not prompter/tui.
- Rule: parallel coders fine while 5-hour usage is low; cut to 1–2 near 70–80%. Small briefs so a stopped coder does not force a re-read.
- Known bug (2026-11-04): prompter summary repeated a stale 추천 from the carried-over handoff note for 3 messages ("ㅇㅇ 알아서 진행해", "ㅇㅇ 실행해", "ㅇㅇ 진행해"). Main ignored it, used latest 추천. Fix: prompter reads 추천 from the latest assistant reply, not the handoff note.
