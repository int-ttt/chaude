# factcheck: thresholds (read-only), 2026-10-29

item1: classifier.go:165-167 new Haiku session
- code: internal/harness/classifier.go:166 `if model == "haiku" && (r.a.Model != "haiku" || rr.Worker != "")`; :167 `r.switchSession("haiku", rr.Worker)`
- rr.Worker is a pool candidate session id, set only for kind=edit (classifier.go:55 comment, :21 prompt)
- Worker set -> resumes that stored Haiku session (switchSession resume=id, classifier.go:192-215)
- Worker empty and current model not haiku -> switchSession("haiku","") -> new session (resumeID="" -> start())
- Worker empty and already on haiku -> branch skipped, stays in current session (no new session)
- r.parked["haiku"] is never resumed: haiku branch returns at classifier.go:170 before the parked lookup at :174
- answer: reuses only pool worker candidate (kind=edit); otherwise opens new. parked haiku id unused.

item2: metrics max context per turn
- files: notes/metrics/2026-10-28.jsonl, notes/metrics/2026-10-29.jsonl
- count: stage=="turn" records = 99 (ctx null: 0)
- max "ctx" (= lastCtx basis, metrics.go:56) = 97376 (session a14a3dd8-0799-4888-9ac8-e20ae1f03237, 2026-10-28T17:59:10Z)
- max "result_ctx" (result-event usage) = 189191 (model haiku, record ctx=95438)
- turns with ctx >= 270000: 0
- note: the two fields disagree; which one rotation uses needs confirming (repl.go:515 sets lastCtx from usageLine arg)

item3: 270k / 150k / 400k locations
- 270k rotation (code): internal/harness/repl.go:848 `fs.IntVar(&a.Threshold, "threshold", 270_000, ...)`
- 270k rotation check: internal/harness/repl.go:745-746 `autoOK && cooled && r.lastCtx >= r.a.Threshold` -> r.rotate
- 270k display: internal/harness/usagebar.go:13 `const ctxBarMax = 270_000`
- 270k docs: notes/DECISIONS.md:16 (261028 user rule, >=270000); README.md:24, README.md:35
- 300k stale: notes/DECISIONS.md:14 says "threshold default 300k"; internal/harness/unit_test.go:89 expects CLAUDE_CODE_AUTO_COMPACT_WINDOW=300000
- 400k compact window (code): internal/harness/repl.go:849 `"compact-window", 400_000`; env set at internal/harness/core.go:29-35
- 400k docs: notes/DECISIONS.md:14; README.md:36
- 150k: CLAUDE.md:27 "Auto-compaction (150k window)"; no 150k constant found in *.go, README, DECISIONS (conflicts with 400k code default)
- CLAUDE.md:19 says subagents compact at "100k window" (different context)

item4: where a Haiku context-handoff threshold goes
- existing generic rotation check: internal/harness/repl.go:743-746 (no model gate; applies to every model)
- DECISIONS.md:40 says threshold switching should apply only to Haiku main (Opus/Sonnet keep context); no model gate in code yet
- proposed gate point: repl.go:746 (add r.a.Model == "haiku")
- threshold flag: repl.go:848; category rotation (not threshold): repl.go:740
- constants: usagebar.go:13 (display only, not rotation)
- rotate(): repl.go:562

not found: 150k as code constant; any Haiku-specific threshold constant.
