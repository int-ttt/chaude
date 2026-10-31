# review: plan-usage-efficiency numbers + R7 turnspend readiness (261030, read-only)
## item1 numbers
- est_usd_all 135.56: MISMATCH(actual 135.58) src usage-eff-measure.txt d) ALL total=135.58 (cwrite 48.00, cread 35.50, out 52.05)
- cache_write 48.00: MATCH
- short_spawns 189 / 4.22: MATCH (f)
- first_msg_cw_usd 6.33: MATCH (e)
- scout median 2352: MATCH (b)
- opus-architect mean 6571: MATCH (b first_mean)
- main-only 11.34: MATCH (reconcile sum main_only 11.3387)
- main+sub 82.89: MATCH (sum main+sub 82.8877)
- naive 467.18: MATCH (usage-ur-reconcile.txt)
- per-session-max 43.34: MATCH (usage-ur-reconcile.txt 43.3377); NOTE usage-causes-and-solutions cites 40.09/40.80 for a different basis
- sonnet-analyst 20.68: MATCH (causes file, m0 10-29)
- opus-architect 13.40: MATCH (causes file, m0 10-29)
- note: plan says "of 49.25 usd" for 10-29; causes file m0 total = 49.58, 49.25 is m2 total
## item2 repl.go
- main turn: t0 := time.Now() repl.go:864 (in caller, before r.runTurn at :871); row logged repl.go:895-897 stage=turn (no source key)
- runTurn repl.go:473; handle closure repl.go:498; ev/message vars: t (type) repl.go:499, isMain repl.go:503, m := asMap(ev["message"]) repl.go:520, blocks repl.go:528
- events loop: r.sess.Events(300s, r.intr, handle) repl.go:576
- assistant case: repl.go:519 `case t == "assistant" && isMain:`; usage lastUsage repl.go:521-522; r.delegs++ repl.go:532-533 (per event, not per id)
- subagent events: handle called for all events (repl.go:500 tracker.onSubEvent first); isMain=false so repl.go:519 case skipped -> sub assistant usage NOT seen in runTurn
- background row: repl.go:407 t0 := st.t0 (fallback time.Now()), fields repl.go:411-412, result_* keys repl.go:416-418, log repl.go:419
## item3
- turnspend.go: not present (ls: No such file)
- grep -i turnspend: only in notes/runs/261030-plan-usage-efficiency.md:47,49. No type turnSpend / newTurnSpend in code.
## item4 R7 risks
- stream dup: assistant event per content block, same message.id; tool_use count per event would double count spawns (existing r.delegs++ at repl.go:532 has same issue)
- spec says onAssistant "if !isMainEvent: store usage" but also "tool_use Agent/Task (main or sub)" -> main tool_use skipped by that branch; spec internally inconsistent
- spawn dedupe needs tool_use id, not per event
- subagent-launched Agent tool_use appears with parent_tool_use_id set; counted only if onAssistant runs on sub events, which runTurn's isMain gate prevents unless hook placed before gate
- fields() merge only in main turn row (repl.go:895); background row (repl.go:411) not covered
- usage dedupe "keep last" per message.id: sub usage may be partial in early stream chunks; last-seen is the final one only if the last event carries full usage
- sub events arrive via same stream (repl.go:500); sub turns after main result may arrive after result -> missing if collected only before result-break
