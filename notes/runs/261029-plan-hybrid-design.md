# Plan: hybrid chaude design (Opus main restored in part + Haiku overview)

date: 261029
status: rough plan, document only, no code edits
inputs: CLAUDE.md (policy block, claude-lean), notes/DECISIONS.md, notes/runs/261029-plan-design-revisit.md, notes/runs/261029-diag-background-panel.md
facts file: notes/FACTS.md has no entries (empty), nothing to cite from it

## Background
- user: "Opus 세션은 원래 정책대로 유지하고, Haiku가 대략적인 계획/개요를 관리하며, 과거 구조와 현재 구조를 부분적으로 합친다(하이브리드)."
- user: "긴 컨텍스트 사용량 때문에 설계가 지금처럼 되어 있다." (source: revisit doc section 3 quote)
- interpretation (사용자 확인 필요): "Opus 세션은 원래 정책대로 유지" = the Opus-main policy text in CLAUDE.md lines 2-27 comes back for the cases listed in (c). It does not mean Opus main for every turn.

## Decisions (4)
- D1 main model (answered 2026-10-29, DECISIONS last row): Haiku stays default main (DECISIONS:38, CLAUDE.md:31). Opus main is used only for design turns, as the original policy says. The context threshold T applies only to the Haiku main; Opus and Sonnet mains keep their context and do not switch by threshold. T value = 사용자 확인 필요. 270k rotation for Opus/Sonnet mains = 사용자 확인 필요 (DECISIONS:16).
- D2 overview: Haiku writes and keeps the rough plan/overview file (`notes/runs/<YYMMDD>-plan-<topic>.md`). Opus and Sonnet only review it and write improvements (CLAUDE.md:31 steps 1-3). Subagents stay for all file/web work.
- D3 order: background-panel fix (step P1-P3) ships first, before any structural change (revisit doc section 6, step 1).
- D4 TUI (answered 2026-10-29): GoLand terminal input issue resolved; chaude runs there and text input works.

## (a) Past design to restore (Opus main, from CLAUDE.md:2-29)
- main thread listens and routes, no work tools, hook-enforced (CLAUDE.md:4). Restore only for the Opus-main case in D1.
- one delegation round per user message, all agent calls in one message, foreground (CLAUDE.md:15).
- `opus-architect` only for an ESCALATE path or when the user says `opus로` (CLAUDE.md:9).
- reply style: natural Korean 존댓말 colleague tone, what changed / what the numbers mean / what is uncertain (CLAUDE.md:20).
- `추천:` final line rule for replies that propose or ask (CLAUDE.md:21).
- no `/clear` or `/compact` suggestions; reset via `/exit` then `claude` (CLAUDE.md:26).
- compact instructions block (CLAUDE.md:28-29): keep open workstreams, pending 추천: default, active agent IDs, final file paths.
- not restored as-is: "Opus delegating to Opus saves nothing" (CLAUDE.md:9) only as a note, not as a blocker, because an Opus main turn is the case that uses it.

## (b) Current design to keep (Haiku main + subagents)
- Haiku-fixed main for small tasks (DECISIONS:38, CLAUDE.md:31).
- large-task flow: Haiku plan -> opus-architect/sonnet-analyst review (improvements only) -> Haiku merge -> Haiku execute -> sonnet-reviewer (CLAUDE.md:31).
- sonnet-reviewer before anything is submitted or printed (CLAUDE.md:9).
- results written to `notes/runs/`, agent returns path + ≤3-line summary (DECISIONS:36, CLAUDE.md:19).
- research agents stay subagents (DECISIONS:25).
- routing: sonnet = 검토·분석, haiku = 간단한 조사·질문, scout/researcher effort high (DECISIONS:35).
- agent model pin hook: subagents may call sonnet-reviewer only (DECISIONS:35).
- category rotation and 270k rotation (DECISIONS:16, :18, :29).
- one-shot Sonnet classifier tags size only (DECISIONS:32, :33).

## (c) Hybrid split by task size and context length
Rules (proposal, unverified):
1. small task (no `[routing: large ...]` hint): Haiku main, Haiku agents. Same as now.
2. large or design task, session ctx < T: Opus main takes the design turn under the restored policy (a). Haiku still writes the overview plan first if the work is multi-step.
3. large or design task, ctx >= T: Haiku main writes the plan, Opus reviews it as opus-architect (current flow). No Opus main.
4. ctx >= 270k: rotate with handoff (DECISIONS:16, :18), any mode.
5. category switch (DECISIONS:29): rotate after the turn, as now. Opus main sessions follow the same rule.
6. Haiku overview file rule: one file per workstream, rewritten each turn; Opus main reads only that file plus the last turn, not the full history. Length cap for the overview = 사용자 확인 필요.
7. constraint on sonnet-analyst (user, 261029, token saving): sonnet-analyst never reads files directly. Group similar files, max 3개 파일/묶음 per group; ask Haiku to condense each group and return only the summary; sonnet-analyst analyzes the summaries. Opus session policy (a) unchanged; Haiku keeps the rough plan; hybrid split per this section.

Context numbers in the repo conflict and must be confirmed before use (see g4):
- auto-compaction 150k window (CLAUDE.md:26)
- subagent compaction 100k window (CLAUDE.md:18)
- child auto-compact window 400k, must stay above threshold (DECISIONS:14)
- rotation 270k (DECISIONS:16)

Cost reasoning used (from revisit doc section 2, general principle, not measured here): main history is re-billed every turn; Opus main costs more per turn than Haiku at the same length, so Opus main is limited to short contexts (rule 2).

## (d) Background-panel fix (step, first)
Cause (diag doc, sections 1-2): drain.go marks an Agent call as not background unless `run_in_background` is true. Current Agent calls omit the flag and are async by default. Their first tool_result is "Async agent launched...", so drain.go marks them Done at launch. The panel's running list is empty.

- P1: in onToolResult (drain.go:72 per diag doc), if the tool_result text starts with "Async agent launched", set Background=true and do not set Done. Also treat a missing flag as a background candidate (diag doc, fix location).
- P2: finishOldestBackground (drain.go:116) and onNotification (drain.go:84) must also settle flagless agents, so a lost completion notice does not leave an agent "running" forever (diag doc section 4, lesson 3).
- P3: tests. Existing panel_test.go and drain_test.go always pass `run_in_background:true`. Add a fixture with the real shape (no flag, "Async agent launched" result) and a sync-result case (diag doc, unverified item 3).

Acceptance P1-P3:
- go test for the new fixtures passes; old tests still pass.
- live TUI check: an agent shows in the panel below the input while running and leaves it on task-notification. Not verified yet (diag doc says no live run).

## (e) Doc cleanup
- CLAUDE.md:2-14 (Opus-main paragraph): mark as "강제 Opus 메인일 때만" section. It conflicts with CLAUDE.md:31 (Haiku-fixed) on who is main. Do not delete; D1 restores part of it.
- CLAUDE.md:31: keep. Add one line for the T rule (rule 2-3 in c) once T is set.
- DECISIONS.md: mark superseded rows with a pointer, not delete:
  - line 30 (conductor delegation to sonnet-session) -> superseded by line 32.
  - line 35 ("main stays single Opus") -> superseded by line 38.
  - line 5 is kept (structure), no change.
  - revisit doc names a "target=haiku|sonnet|opus" design row (revisit section 3). That exact wording is not found in DECISIONS.md lines 1-39, so confirm before marking (g5).
- remove the phrase that Haiku escalates difficulty itself (revisit section 3, discarded list). Classification is one Sonnet call (DECISIONS:32).
- classifier prompt shrink: only after metrics stage=classify is summed (revisit section 3). Not part of this plan's first pass.
- FACTS.md: no change (empty).

## (f) Acceptance criteria per step
- P1-P3: see (d). Pass = fixtures green plus live panel check when the user allows a TUI run.
- E1 (CLAUDE.md split): grep shows the Opus-main section labeled as forced-only; grep for "Opus 메인" outside that label returns nothing.
- E2 (DECISIONS markers): lines 30 and 35 each carry a pointer to line 32 and line 38; no other rows changed.
- H1 (ctx rule T): with ctx below T and a large-tagged prompt, the turn runs with Opus main; with ctx at or above T, Haiku main. Check via metrics model field per turn.
- H2 (overview file): one overview file exists per workstream; Opus main turn reads it and does not reread the full history (check by transcript size per turn in metrics, value to compare = 사용자 확인 필요).
- R1 (rotation): at 270k the handoff and switch still run in every mode (regression test of existing rotation).
- all steps: `grep` each target file for the new text and confirm the old text is gone where a replacement was made.

## (g) Open questions (사용자 확인 필요)
1. "Opus 세션은 원래 정책대로 유지" : Opus main only for design turns (D1), or Opus main again as default?
2. T (switch threshold): answered 2026-10-29, applies only to Haiku main (see D1). Value = 사용자 확인 필요.
3. overview file length cap: 사용자 확인 필요.
4. which context number governs main: 150k compact (CLAUDE.md:26), 400k child window (DECISIONS:14), or 270k rotation (DECISIONS:16)?
5. the "target=haiku|sonnet|opus" design row named in the revisit doc is not in DECISIONS.md; which row is meant? 사용자 확인 필요.
6. may the live TUI be run for the panel check (P-acceptance)? The diag doc did not run it.
7. the revisit doc concludes that the subagent design still holds (revisit section 1). That is an opinion from a general cost model, not measured. Accept as a working assumption, or measure first with metrics? 사용자 확인 필요.
