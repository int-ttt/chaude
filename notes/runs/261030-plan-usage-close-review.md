# review: 261030-plan-usage-close (haiku-checker; sonnet-reviewer blocked by hook for opus subagents)
- refs: idleTurn repl.go:347 (reply,usage,t0,active,streamed,sawNotif); idleFinish call site :341; fg fields :901-909.
- turnSpend: sub_* only from events with parent_tool_use_id; spawns counts Agent/Task tool_use from any assistant event; keys: spawns(int), spawn_types(map), sub_msgs, sub_input_tokens, sub_output_tokens, sub_cache_creation_input_tokens, sub_cache_read_input_tokens; absent when zero -> default 0.
- turnCostUSD usagepoll.go:113: first turn of session returns full cost; resets on session change or c<prev -> S2 formula tautological; use per-row check cost_turn_usd == cost_usd - prev cost_usd (same session, sorted by ts), count resets separately.
- tests: &Repl{metrics: newMetrics(dir,nil)} + read dir/YYYY-MM-DD.jsonl (ratestatus_test.go:14-30); idleFinish needs &Session{}, non-nil Transcript (idle_test.go:42); check pollUsage does not hit network.
- S4: add --include-partial-messages; input line {"type":"user","message":{"role":"user","content":"<text>"}}; drop second "done" turn (notification turn is automatic); skip S5 if no post-result events.
- order: S1 test before S4; S5 after S4; S6 last.
