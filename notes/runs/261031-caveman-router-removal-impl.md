task: caveman-structure.md 기준 라우터 완전 삭제 + haiku main -> opus-architect 전달 (261031)
base: caveman S0~S5 already in master (27c60e2 v0.9.0); prior dirty impl (router body deletion, sonnet agents deleted, haiku 80k/opus 200k) kept as baseline, not redone
changed:
- internal/harness/repl.go: Conductor field/flag/setConductor//mode conductor removed; metrics "conductor" keys removed; + const forwardRule, func forwardRuleFor(model) ("" if model contains opus); start() appends forwardRule to --append-system-prompt
- internal/harness/tui.go: /mode help = "permission mode: default|acceptEdits|plan|auto (shift+tab cycles)"; conductor branches removed
- internal/harness/orch.go: orchRuleB/C/E sonnet -> opus-architect wording
- internal/harness/classifier.go -> session_switch.go (git mv, content same)
- internal/harness/budget.go:42, resumeshift_test.go:63 comments; delegation_test.go + TestForwardRuleFor
- .claude/hooks/main-thread-guard.py:32, caveman-format.py:11 TARGETS, haiku-context-guard.py:4-6,29 sonnet text removed
- .claude/agents/haiku-prompter.md:12, haiku-coder.md:13, opus-architect.md:3,12,15 sonnet refs removed (R-decide block byte-identical to CAVEMAN_FORMAT.md 3.4)
autoCompactWindow: user "claude에만 적용" -> .claude/settings.json autoCompactWindow 150000 = plain claude only, unchanged; chaude child uses -compact-window 400_000 env (core.go:35) so opus 200k rotation reachable. DECISIONS:47 open item closed.
checks: gofmt -l empty; go build/vet rc0; go test ./... -count=1 ok chaude/internal/harness 0.638s; python3 -I -m unittest discover -s tests/hooks Ran 10 OK; haiku-checker 7/7 PASS
left (main thread): CLAUDE.md:8-13,16,28,33 stale sonnet/classifier text (CLAUDE.md not edited by subagents); conductor_test.go filename kept; ledger.py:41,94 + budget.go/usage_report sonnet price/tier parsing kept (logic); haiku-context-guard env CLAUDE_HARNESS_ALLOW_SONNET_IO name kept; agent-model-pin.py:10 comment; sonnet-reviewer review not run (main thread)
risk: forwardRule chosen at session start from r.a.Model; /model switch mid-session keeps old rule until next start/rotation
