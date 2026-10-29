# haiku-conductor-plan
changed: internal/harness/classifier.go (pickModel/pickTarget haiku-fixed, planHint, route no restart), repl.go (comments/flag text), conductor_test.go (policy+TestHaikuPlanWorkflow), CLAUDE.md, .claude/agents/opus-architect.md, sonnet-analyst.md, notes/DECISIONS.md
hook agent-model-pin.py: unchanged (main thread may call opus-architect/sonnet-analyst; subagents still limited)
unverified: live classifier (CHAUDE_LIVE=1) not run; Haiku following planHint in practice not tested
inference: classifier prompt still outputs kind/worker; harmless, worker resume only when matching candidate
