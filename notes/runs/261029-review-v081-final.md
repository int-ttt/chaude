verdict: APPROVE (0 blocking)
prior blocking: router default flip -> resolved (user approved; CHANGELOG bullet added; classifier.go comments updated; 3 tests updated)
build/vet/test: pass
changelog vs diff: matches (resume onUserEvent in drain/repl/livechild/children; panel wheel tui/agentview; empty-reply result fallback repl.go; sonnet-analyst tools; opus Agent-only; router haiku; version 0.8.1)
non-blocking:
- CHANGELOG omits CLAUDE.md policy change (final answer content in reply, not path only); optional 1 line
- CHANGELOG "Read/Grep/Glob/Edit 도구 제거": true for tools list (Write,Bash remain); ok
- haiku-context-guard.py:22 Grep/Glob deny dead: .claude/settings.json:70 matcher is "Read" only; tool-list removal covers it
- agentview.go panelWheel clamp loose (unverified)
commit (v0.8.1):
- CHANGELOG.md
- cmd/chaude/main.go
- internal/harness/{agentview,children,classifier,drain,livechild,repl,tui}.go
- internal/harness/{classifier_model_test,conductor_test,panel_test,routerprompt_test}.go
- CLAUDE.md (output-mode + Opus no-file policy; changelog-related)
- .claude/agents/opus-architect.md, .claude/agents/sonnet-analyst.md, .claude/hooks/haiku-context-guard.py
leave out:
- notes/FACTS.md (API pricing, 0.8.0 budget data, unrelated), notes/LEDGER.md (auto log), notes/research/billing.md (untracked), notes/runs/*, notes/metrics/*
