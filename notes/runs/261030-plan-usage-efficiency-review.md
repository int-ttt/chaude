# review: 261030-plan-usage-efficiency (haiku-checker; sonnet-reviewer blocked by hook v0.8.2)
numbers: 10/11 match; est_usd_all 135.56 -> 135.58 (fixed); 49.25 is m2, m0 49.58 (annotated)
hook_site: repl.go runTurn :473, handle closure :498, isMain :503, assistant case :519 gated by isMain -> sub usage dropped today; turn row :895-904 in caller (t0 :864)
fix1: call spend hook at top of handle (before switch) so non-main assistant events are seen
fix2: dedupe spawns by tool_use id (existing r.delegs++ at :532 may double-count repeated events)
fix3: usage dedupe per message.id takes per-field max, not keep-last (partial usage in early blocks)
fix4: spec said usage non-main only but spawns main+sub: intended; spawns counted at any level, sub launches carry parent_tool_use_id
fix5: store turnSpend on Repl (r.spend), reset at runTurn start, merge in caller before Log at :904
gap: background turn row (:411) not covered in this change; noted
detail: notes/runs/261030-review-r7-spend-check.md
verdict: plan OK after fixes 1-5; R1-R6 are practice items, only R7 implemented
