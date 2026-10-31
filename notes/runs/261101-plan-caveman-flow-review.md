# review: 261101-plan-caveman-flow.md
verdict: approve with 6 fixes
- R1 risk: tests/hooks may assert R-write REQUIRED (changed,diffstat,check,ws). S6 hook change must grep tests/hooks for "diffstat" first and update the same tests; run tests/hooks after
- R2 risk: until S6, caveman-format.py (log mode, E5) records missing diffstat/check/ws for every new-style write return. expected noise, not a failure; do not switch CAVEMAN_GUARD to warn before S6
- R3 gap: agent files still define old R-write until S6. bridge: opus/main briefs to coder/batch-worker must say in limits "return: status + changed(경로: 바꾼 부분) only, no diffstat/check/summary_ko"
- R4 gap: haiku-prompter loop returns per-round log; relay shape for loops not defined. keep current loop return; out of scope
- R5 risk: relay adds main tokens (read relay up to 1500 chars) vs Haiku main rotation 80k (CLAUDE.md:13). keep cap; trim claims first
- R6 open_q: "출력창에는 caveman 결과 요약 후 표시" read as haiku-tui 존댓말 요약 then item list. if user wants raw caveman lines shown too, add a raw block after summary
