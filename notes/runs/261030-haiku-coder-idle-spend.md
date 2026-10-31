task: idle/background turn spend row (S1) + usage_report spend and cost_turn check (S2)
status: done
changed:
- internal/harness/repl.go (idleTurn.spend; idleEvent feeds spend; idleFinish merges spend + result_model_usage; fg result row result_model_usage)
- internal/harness/idlespend_test.go (new; TestIdleSpendRow, TestIdleSpendLiveReplay)
- scripts/usage_report.py (== subagent spend ==, == cost_turn check == sections before bad_lines)
checks:
- gofmt -l internal/ : no output (exit 0)
- go vet ./... : exit 0
- go test ./internal/harness/... -count=1 : ok chaude/internal/harness 0.713s
- TestIdleSpendRow : PASS (sub_output_tokens 7, spawns 1, result_model_usage present)
- TestIdleSpendLiveReplay : SKIP (testdata/bg_turn_live.jsonl missing)
- pollUsage guard: t.Setenv HARNESS_USAGE_POLL=0 in test helper (no production change)
- python3 -I scripts/usage_report.py --since 2026-10-30 : exit 0
report_lines:
rows in window: 200
== subagent spend ==
sub_msgs sum: 3
sub_input_tokens sum: 6
sub_output_tokens sum: 12
sub_cache_creation_input_tokens sum: 15352
sub_cache_read_input_tokens sum: 17662
spawns sum: 1
spawn_types: haiku-scout=1
row counts per source: (none)=54, background=22
== cost_turn check ==
cost_turn check: rows=3 mismatched=0 resets=0
bad_lines: 0
doubts:
- cost_turn check uses full unwindowed chain per session (baseline before window); counts only rows in window.
- rows with no time are skipped in cost_turn check.
- source "(none)" label used for fg turn rows that have no source key.
- fg result_model_usage not exercised by a test.
- no commit made.
