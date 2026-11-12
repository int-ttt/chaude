# Confirmed Facts
<!-- One line each. Only facts the user stated or confirmed. Update immediately on correction. -->
<!-- format: - [area] fact (source: user / file path) -->
<!-- areas e.g.: people / project / env / schedule / constraint. Terse English; keep Korean values verbatim -->

## API 가격 (USD/MTok: input / 5m write / 1h write / cache read / output)
- 출처: https://docs.claude.com/en/docs/about-claude/pricing (platform.claude.com 으로 리다이렉트), 확인일 2026-10-29
- Opus 5.5: 4 / 5 / 8 / 0.20 / 20
- Sonnet 5.5: 2 / 2.50 / 4 / 0.10 / 10
- Haiku 5.5 (100k 이하): 0.10 / 0.125 / 0.20 / 0.01 / 0.50 (100k 초과: 0.50 / 0.625 / 1 / 0.05 / 2.50)
- budget.go 별칭 가정: opus=Opus 5.5, sonnet=Sonnet 5.5, haiku=Haiku 5.5
- [agent-rename] 2026-11-12: agent name haiku-tui -> haiku-output (old -> new). Live files use haiku-output; old name stays in notes/DECISIONS.md, notes/archive, plan/archive, notes/research/subagent-usage.md, notes/runs, CHANGELOG past entries.
