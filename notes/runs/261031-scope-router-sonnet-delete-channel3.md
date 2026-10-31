task: write file verbatim
paths: /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261031-scope-router-sonnet-delete-channel3.md
ctx: content below, save exactly
limits: no other files
return: R-write, 1 line

found:
- user req: delete 기존 라우터 및 sonnet 관련 모델 전부 + implement plan/caveman-structure.md 2절 (채널 3층)
- plan 2절 L1 row (plan/caveman-structure.md:~36-40): "L1 지휘 채널 | 메인 <-> opus-architect / sonnet-analyst / sonnet-reviewer"
- plan 14절: no item decides removing sonnet-*; U1~U10 all decided/unrelated
conflict:
- deleting sonnet-analyst/sonnet-reviewer + .claude/agents sonnet defs + agent-model-pin.py sonnet pin + settings.json sonnet model removes 2 of 3 L1 endpoints that 2절 requires
- 2절 L0/L1/L2 otherwise appear already present (v0.9.0 caveman S0~S5, haiku-tui, haiku-prompter exist); scope of "implement" unclear without sonnet L1
options:
- A: delete router only (classifier.go, haikupool.go, routerprompt_test.go, classifier_model_test.go); keep sonnet-*; implement 2절 as written
- B: delete router + sonnet; amend plan 2절 L1 to "메인 <-> opus-architect" only; then implement (needs plan edit, user approval)
- C: delete router + sonnet; L1 = opus-architect only plus haiku-*; redefine L1/L2 hops (design change, rule_3 relay count check)
open_decision: which of A/B/C (user only)
not_done: no files deleted or changed; no tests run
not_checked: whether other code (go packages, hooks tests) depends on classifier.go/haikupool.go
