# sonnet conductor (v0.6.7) - PARTIAL
done (1): markerFilter strips [cat:]/[cat-switch:] lines from displayed stream (category.go; reply text kept raw for parsing); test TestMarkerFilter
live check (haiku, 2 prompts): markers emitted (stage=category from ""->"파이썬 리스트", then switch ->"김치찌개 레시피"), marker not displayed, rotate #1 fired with reason "category ..." (handoff_valid true, attempts 1). cost $0.0024 + $0.0034.
not done: conductor mode (flag + /mode, Sonnet main, escalation to Opus, metrics tag of conductor model), live A/B, bump/tag v0.6.7, release, install.
reason: scope exceeds response cap; plan notes/runs/261028-plan-sonnet-conductor.md is 52KB, not yet read past §3.
next: read plan §1,§4,§7; design routing prompt (append-system-prompt) + --conductor flag + metrics field "conductor"; then A/B.
