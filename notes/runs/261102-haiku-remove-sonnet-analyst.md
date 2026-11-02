task: remove sonnet-analyst (agent refs, skill step, docs)
status: done
agent_file: none in tree (.claude/agents, src/main/intt embeds, clean.go lists): no file to delete
changed:
- .claude/skills/distill/SKILL.md: merge step -> Opus main decides + haiku-batch-worker file work; explain step -> Opus text + haiku-batch-worker save; "Sonnet merge" -> "Opus merge decision"
- docs/claude-lean/FRESH_START.md:47: merge -> 메인 Opus 기준 + haiku-batch-worker
- docs/claude-lean/README.md:13,46 and line 3 routing: sonnet-analyst removed; judgment -> 메인 Opus
- docs/claude-lean/APPLY_GUIDE.md:52,85,154: sonnet-analyst removed
- CHANGELOG.md Unreleased: 1 line added
kept (history): CHANGELOG.md:109,119,129; docs/claude-lean/CHANGELOG.md:10,11,92,102; notes/, record/, .chaude/history.jsonl
check: grep -ril sonnet-analyst (excl notes/record/.chaude/.git) -> CHANGELOG.md, docs/claude-lean/CHANGELOG.md (history only)
check: cd src/main/intt && go vet ./... -> rc 0; go test -count=1 ./... -> all ok (clean, harness, scripts, settings)
doubts:
- sonnet-reviewer still named in README/APPLY_GUIDE; no agent file exists; Sonnet tier removed 2026-10-31 per agent-model-pin.py. Not touched (out of scope).
- Opus main is blocked from Read/Write by main-thread-guard.py, so merge file IO goes to haiku-batch-worker.
