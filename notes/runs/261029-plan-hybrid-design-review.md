# Review: 261029-plan-hybrid-design.md (improvements only, nothing executed)

reviewer: opus-architect, 261029
read: plan, revisit doc, diag doc, DECISIONS.md, LESSONS.md, CLAUDE.md, classifier.go:29-213, repl.go:319/745/848-849, core.go:29-35, .claude/settings.json:4, main-thread-guard.py, haiku-context-guard.py:7
metrics used: notes/metrics/2026-10-28.jsonl + 2026-10-29.jsonl (94 turn, 38 classify, 14 rotate, 54 category rows)
metrics caveat: turn `cost_usd` = result.total_cost_usd (repl.go:319) = cumulative per session, includes subagent spend. Per-turn cost below = session max cost / session turns. Task mix differs by model, so cost numbers are indicative, not controlled.

## metrics facts (basis for items 3-5)
- haiku main: 7 sessions, 72 turns, ctx p50 48.2k, p90 79.3k, max 97.4k; $0.056/turn
- opus main: 7 sessions, 14 turns (1-4 per session), ctx max 44.5k; entry cache_creation 20.4k-28.2k tokens; first turn ~$0.33; $0.304/turn
- sonnet main: 6 sessions, 8 turns, $0.459/turn
- baseline ctx of a fresh main session: 33k-41k
- growth: ~2.0k ctx/turn (session a14a3dd8: 34.7k -> 97.4k over 31 turns)
- no turn ever at ctx >= 100k; so 150k/270k/400k never fired in practice
- rotate reasons: "route opus" 7, "route sonnet" 6, category 1; category switch 1 of 54 category rows

## improvements

1. g5 is answerable from the repo; remove it.
   fact: the "target=haiku-*|sonnet-analyst|opus-architect" row is DECISIONS.md:30. "main stays single Opus" is the tail of :35. Classifier-routes-to-opus (:33) is also superseded by :38.
   plan edit: (e) DECISIONS bullet -> "line 30 -> superseded by :32; line 33 (routes to opus) -> superseded by :38; line 35 tail 'main stays single Opus' -> superseded by :38". Delete g5 and the "not found" sentence in (e).

2. g4 is answerable from the repo; replace the "numbers conflict" block.
   fact: 150k = .claude/settings.json:4 `autoCompactWindow` (plain `claude`, outside chaude). chaude child gets CLAUDE_CODE_AUTO_COMPACT_WINDOW=400000 (repl.go:849, core.go:35) which overrides it. Rotation = --threshold 270000 checked after each answer (repl.go:745, :848). 100k = subagents (haiku-context-guard.py:7). In chaude the main is governed by 270k; 400k is only a safety net above it.
   plan edit: (c) replace the 4-line conflict list with this mapping; drop g4. Add one unverified item: the model's native context window may be below 270k/400k for some main models; if so rotation never fires before native limits. Check once (no metric reached 100k, so untested).

3. D1 "session ctx < T" is ill-defined, because switching to Opus is itself a session change.
   fact: Haiku -> Opus = rotate("route opus") with handoff (classifier.go:178-186) or resume of parked Opus (classifier.go:172-176). The Haiku session's ctx does not carry over; a new Opus session starts at ~35-41k.
   plan edit: D1 and (c) rules 2-3 -> "T is measured on the Opus main session itself (current or parked). Enter Opus main when size=large and (no parked Opus, or parked Opus ctx < T). While in Opus main, after each answer: if ctx >= T, leave Opus main (park/close, return to Haiku with notice, see item 6); do not rotate Opus->Opus." Rule 4 -> "270k applies to the Haiku main; with T < 270k it never fires for Opus main. Post-answer check order: T check first, then 270k (repl.go:745)."

4. T can be made concrete; propose a default instead of leaving it open.
   proposal: T = 80,000 as flag `--opus-main-max-ctx` (default 80000, 0 = Opus main off = current behavior).
   why: entry ~40k + ~2k/turn -> ~20 design turns before leaving, covering the revisit doc's 5-10 turn crossover. Haiku p90 is 79k, so 80k matches observed working depth. Any T >= 100k never triggered in 94 turns, so rule 3 would be dead code; T >= 150k is equivalent to "no T".
   plan edit: D1 "T = 사용자 확인 필요" -> "T = 80k default (flag), user may change". g2 -> "80k 기본값으로 시작해도 될까요?" (user confirms value only).

5. The switch-in overhead is missing from the cost reasoning; add a stickiness rule.
   fact: every new Opus session paid a 20-28k-token cache write (first Opus turn ~$0.33 vs Haiku $0.056/turn); plus a handoff turn on the session being left. An opus-architect review subagent also pays a fresh baseline per call, so Opus main only wins from the 2nd consecutive design turn on (estimate).
   plan edit: add (c) rule 2a: "Enter Opus main only if (a) user says opus로, or (b) size=large and previous routed prompt was also large/same topic, or a parked Opus session exists under T. A single isolated large prompt uses the current flow (Haiku plan + opus-architect review)." Rule 2b: "Once in Opus main, keep it for topic=same prompts of any size (pickModel(cur=opus) already keeps opus and adds delegateHint for topic=new&small, classifier.go:42-43); leave only on ctx >= T, category switch, or topic=new&large non-design."
   remove from (c): the general cost sentence "Opus main costs more per turn... limited to short contexts" -> replace with the measured numbers above and "re-billing at cache-read price is small at <=80k; the dominant cost is session entry and >1h cache expiry (item 11)".

6. Return path Opus -> Haiku loses context; add a step before H1.
   fact (read, not run): route() for model=haiku calls switchSession("haiku", rr.Worker) (classifier.go:165-167). With Worker="" resumeID is empty and parked["haiku"] is not consulted (only consulted for model != haiku, :172), and no handoff is made. The Haiku session left by rotate("route opus") is also not parked. Result: a fresh Haiku main that knows neither its earlier history nor the Opus turn's outcome (unless pendingSnapshot happens to hold it; unverified).
   plan edit: new step H0 (before H1): "Haiku -> Opus: park the Haiku session (switchSession, not rotate) . Opus -> Haiku: resume parked['haiku'] and queue notice '[opus 설계 완료] overview: <path> · 추천: <first item>' prepended to the next input (same mechanism as drain notices, DECISIONS:18)." Acceptance H0: test that after haiku->opus->haiku, resumeID == original Haiku session id and the notice is in the next sent text.

7. D2 conflicts with the main-thread hook: no main session can write files.
   fact: main-thread-guard.py blocks Write/Edit/Bash for any main session (no agent_id), regardless of model. planHint step (1) "write the plan yourself" (classifier.go:31) already has this conflict.
   plan edit: D2 -> "The overview file is written and rewritten by one continuing `haiku-drafter` instance per workstream (CLAUDE.md:17 continue rule); main (Haiku or Opus) only passes the path." Add to (e): fix planHint wording "(1) write ... yourself" -> "(1) have haiku-drafter write ..." (code edit, separate step, haiku-coder).

8. Rule 2 ordering "Haiku still writes the overview plan first" cannot happen inside an Opus main turn.
   fact: route() switches the session before the turn; restored policy (a) allows one delegation round per user message.
   plan edit: rule 2 -> "In the Opus main turn, Opus decides from the overview file + last turn, and in its single delegation round includes the haiku-drafter overview update (and implementation agents). No separate Haiku pre-step." Drop "first".

9. Restored policy (a) needs an explicit "no self-review" line for Opus main.
   risk: (b) large-task flow says opus-architect reviews; in Opus main that is Opus reviewing Opus (CLAUDE.md:9 "saves nothing").
   plan edit: (a) last bullet -> "In Opus main mode the (b) plan->review flow is not used: Opus decides itself; sonnet-analyst/Haiku agents implement; sonnet-reviewer reviews. opus-architect only for ESCALATE." Code already adds planHint only when model==haiku (classifier.go:94), so no code change; add a test asserting no planHint when cur=opus.

10. Category rotation inside Opus main creates another Opus baseline write.
   plan edit: rule 5 -> "On category switch in Opus main, return to Haiku (item 6 path) instead of rotating to a new Opus session." Priority low: 1 switch in 54 category rows.

11. Parked Opus resumed after cache expiry rewrites the full context at Opus write price.
   fact: cache TTL pinned 1h, idle rotate 3599s (DECISIONS:23).
   plan edit: add (c) rule 7: "A parked Opus session idle >= idle-rotate time is dropped; the next Opus entry starts fresh with the overview file only." Also note: r.parked is in memory; whether it survives chaude restart is unverified (0.7.0 persisted subagent state, not parked sessions).

12. Acceptance criteria need metric fields that exist and the cumulative-cost caveat.
   plan edit (f):
   - H1: check classify `routed_model`, turn `model`, rotate `reason` ("route opus", new "opus ctx>=T"). Pass: with T=80k, no Opus main turn has ctx >= 80k+one-turn growth (~85k); leaving Opus logs the new reason.
   - H2: replace "transcript size per turn, value 사용자 확인 필요" with "Opus entry turn result_cache_creation_input_tokens <= 30k and ctx delta per Opus turn <= 5k (observed Haiku growth ~2k/turn)". Use per-turn deltas of cost_usd, never raw values (cumulative).
   - E1: CLAUDE.md is English; grep `Opus 메인` passes trivially. Use `grep -n "Opus main\|Opus-main\|never main" CLAUDE.md` and expect hits only inside the forced/hybrid section. CLAUDE.md:31 "Opus is for design only, never main" must be reworded for the hybrid, else E1 fails.
   - R1: also test that a 270k check never fires on an Opus main when T < 270k (T check precedes).

13. P1 wording: drop "treat a missing flag as a background candidate".
   risk: combined with the prefix check it is redundant; alone it would keep sync agents (flag absent, real output) running until the fallback.
   plan edit: P1 -> "On the Agent call's first tool_result: text starts with 'Async agent launched' -> Background=true, not Done; else Done (sync). Flag run_in_background:true keeps current behavior." P3 adds both fixtures (already listed).

14. Ordering and gating.
   plan edit: order = P1-P3 (independent, ship first) -> E2 (DECISIONS markers, no user input needed after items 1-2) -> ask g1 -> H0 -> H1 (flag, default T=80k) -> H2 -> E1 (CLAUDE.md wording last, after behavior exists). E1 before g1 risks rewriting CLAUDE.md twice.

## open questions after this review (사용자 확인 필요 only)
- g1 (Opus main for design turns only vs default) - unchanged
- g2 reduced to confirming T=80k
- g3 overview cap: propose 3,000 chars (fits a <=5-line brief + path; ~1k tokens); user confirms
- g6 live TUI run - unchanged
- g7 partly answered by metrics (Haiku $0.056/turn vs Opus $0.304/turn, uncontrolled); measuring Opus-main vs opus-architect on the same task remains the only clean test

## [haiku] steps
none (review only)
