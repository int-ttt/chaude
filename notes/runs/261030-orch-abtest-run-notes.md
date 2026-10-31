# orch A/B run notes (261030)
status: done 60/60, no not_run, cap not hit
jsonl: notes/runs/261030-orch-abtest.jsonl (60 rows)
answers: notes/runs/abtest-answers/<mode>-<task>-<n>.txt (stdout), .err in /tmp/orchab/out/
path: chaude piped stdin, --orch <mode>, --task-budget-usd 0.5 (m0,e) / 1.0 (others); usd from wt notes/metrics (cost_turn_usd, else cost_usd, ts>=run start); measured=true
fallback ab_headless.py: not used
wt: HEAD 1af0f16 + overlay of working-tree internal/ cmd/ scripts/ go.mod (22 files) + .claude/ + notes/DECISIONS.md. Reason: HEAD lacks untracked orch.go/orch_test.go, so T3 seed and test impossible otherwise. First attempt (HEAD-only, 4 runs m0 T1/T2) discarded: /tmp/orchab/discarded/
spend: preflight m0 T5 0.0066 + discarded 0.4050 + jsonl 8.6736 = cum 9.0852 (cap 15)
per mode usd (jsonl): m0 0.2334 | b 2.0947 | c 2.0689 | d 2.0432 | f 2.1439 | e 0.0895
PASS (mechanical, T1/T3/T5): m0 4/6 | b 4/6 | c 2/6 | d 3/6 | f 2/6 | e 6/6
T2/T4: pass=null (rubric judging not done)
T3 pass = go test rc of wt at end of run; some runs ended with subagent still running (answer says "backgrounded"), so FAIL may be incomplete work, not final.
T1 pass = answer stdout contains 150k|150,000; some answers backgrounded the scout before reporting.
spawn_count = sum of metrics stage=turn delegations; looks low for B/C/D/F (8) and doubtful; not verified against Agent tool calls.
