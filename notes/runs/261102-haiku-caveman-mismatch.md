status: partial (build blocked by concurrent edit)
truth: plan/caveman-structure.md
item_a: fixed docs; CLAUDE.md:31 notes/LEDGER.md -> record/LEDGER.md (code ledger.py:96-98 already record/)
item_b: fixed docs; notes/DECISIONS.md bin/harness -> bin/chaude, harness gen-settings -> chaude gen-settings, dist/harness- -> dist/chaude-, `harness repl` -> `chaude repl` (settings.json and install use chaude); internal/harness/ paths and conceptual prose "harness" left
item_c: fixed docs; CLAUDE.md:5 and docs/guide/models-and-agents.md "no Sonnet" scoped to default Haiku-main flow; main_model sonnet/fable noted (code settings_picker.go:18 kept)
item_d: fixed docs; DECISIONS line 16 [rotate] 270k tagged SUPERSEDED by line 47 (haiku 80,000 / opus 200,000; code repl.go:28-29). README/CLAUDE 200k/80k unchanged
item_e: fixed code+rule+tests; resident.go viaPrompter always appends "\n\n[original]\n"+text (U8=c), needOriginal no longer used in prod; rules/resident-prompter.md drops NEED_ORIGINAL; resident_route_test.go expectations for SummaryOnly and OKNoOriginal and NoFlagHeuristic updated
doubt_e: resident prompter has no loop role (Minimal, no tools); U10=b loop role lives only in .claude/agents/haiku-prompter.md (not changed)
doubt: orch.go:46 says "rotating at 40k tokens" for [ws]; not fixed (out of scope)
doubt: budget.go, usage_report.go keep sonnet price tables; harmless, not fixed
check_go_build: go build ./... -> FAIL (internal/harness/tui.go:84:6 resultText redeclared; other decl internal/harness/importview.go:198:6; untracked file from concurrent from-claude import agent; not my edit). Retried 2x, same.
check_go_test_before_conflict: go test ./... -> PASS (internal/harness ok 1.049s, after my edits, before importview.go appeared)
check_go_test_now: go test ./... -> FAIL (build failed, same conflict)
check_python: python3 -m unittest discover tests/hooks -> PASS (Ran 35 tests, OK)
next: rerun go build/test in src/main/intt once importview.go/tui.go conflict is resolved by owning agent
