# impl: caveman 전체 구현 (S0~S5 + haiku-tui) + v0.9.0
status: done (branch only; master merge pending user decision)
plan: notes/runs/261031-plan-caveman-impl.md
worktree: /Users/leekwangseok/PycharmProjects/claude-harness-impl
branch: caveman-impl (base caveman-u8 39db4ad; not merged, not pushed, no tag)
commits (10, caveman-u8..caveman-impl):
- ef90cf9 S0 CAVEMAN_FORMAT.md (plan 3.1~3.6 verbatim, 72 lines)
- 2f2685b S1 R-read templates scout/researcher + .claude/hooks/caveman-format.py + PostToolUse Agent|Task + tests
- d4e4b32 S2 R-write templates (coder, batch-worker, runner, checker, drafter)
- 401e5a1 S3 B형식 + R-decide (opus-architect, sonnet-analyst, sonnet-reviewer)
- ccf2b9b haiku-tui new (U9=a), haiku-prompter output -> haiku-tui
- bcc9f8e S4 TARGETS 12 agents
- ae2cbf0 S5 CLAUDE.md:4 범위 한정, O2 line removed
- 72ebf07 plan 14절/6절/7절 U3·U4·U7 결정됨
- e1f3924 plan :48/:218/:219 보류 wording cleanup
- 27c60e2 release v0.9.0 (cmd/chaude/main.go version 0.9.0, CHANGELOG ## 0.9.0)
decisions_in_impl: U3 matcher Agent|Task; U4 mode f on by default (repl.go:1090) -> ws none allowed; U7 hook reads tool_input.prompt, lang: ko -> summary_ko required
checks (haiku-runner, worktree):
- go build ./...: PASS
- go test ./...: PASS; packages ok 1 (internal/harness), FAIL 0, no-test 5; tests pass 164 fail 0
- py_compile .claude/hooks/*.py: PASS
- unittest .claude/hooks/tests: Ran 14, OK (0 fail)
- settings.json PostToolUse matchers: ['Agent|Task']
- chaude --version: chaude 0.9.0
review: haiku-checker (sonnet-reviewer not callable: hook v0.8.2 blocks opus->sonnet). 5/6 PASS; 1 FAIL (plan 보류 wording) fixed in e1f3924
open:
- master merge: master has 39 uncommitted files (v0.8.2 work incl. same agent/hook/settings/CLAUDE.md/CHANGELOG) -> commit or stash first; expect conflicts in .claude/agents/*.md, settings.json, CLAUDE.md, CHANGELOG (0.8.2 vs 0.9.0 order)
- sonnet-reviewer review not done (main session should run before merge)
- haiku-extractor, haiku-prompter have no caveman template (extractor out of plan; prompter in TARGETS but hook skips briefs without return: line)
- stale worktree /Users/leekwangseok/PycharmProjects/claude-harness-caveman (caveman-relay-impl, uncommitted old design) untouched
