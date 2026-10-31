# review: notes/runs/261031-plan-impl-gap.md
date: 2026-10-31 ; reviewer: opus-architect ; scope: improvements only, not executed
verdict: A1/A2 keep (merge, reword); A3 drop code change; B2 close as explained; resolve 미확정 before A

## decisions
D1 A2 vs §4: no conflict if verbatim kept. evidence: haiku-prompter.md:11 input = "user verbatim, target agent, task brief"; caveman-format.py:20,96-99 already checks main->prompter brief for B keys; §4 :133 "받는 형식: 사용자 원문" describes payload, not envelope.
  - fix A2: B형식 = envelope; user text rides inside as `ctx: user_verbatim: |` block (verbatim, no summary edit). outgoing relay to opus unchanged (prompter.md:29-34 user_verbatim + summary).
  - fix A2: required keys = 5 (task, paths, limits, return, lang) to match B_KEYS caveman-format.py:20; ctx optional. plan says 6 -> wrong, rewrite "5 필수 + ctx 선택".
  - add step: §4 :133 받는 형식 -> "B형식 (ctx에 사용자 원문 verbatim)". executor haiku-batch-worker.
D2 I14: explain only, no code change. evidence: b4855b7 main-thread-guard.py:32 = reason text only; agent-model-pin.py = sonnet tier removed (SUB_CALLERS, prompter targets) per user policy DECISIONS.md:49 + LESSONS.md:5.
  - fix B2: status PARTIAL -> DONE(사용자 결정 반영); §9 "동작 안 바꿈" = this plan's impl scope, not prior user-decided policy. add note to plan :272 citing DECISIONS.md:49. remove "사용자 확인 필요" unless user disputes.
D3 A3: template only, not orchws.go. evidence: routeOrch call removed in b4855b7 (repl.go:396), only def orchws.go:78 + test remain -> parseWS has no live path; parseWS reads input tag "[ws:<id>]", not R-write field "ws: none"; template already has `ws: <작업 공간 id | none>` in haiku-checker:27, runner:29, coder:29, drafter:31, batch-worker:28.
  - fix A3: delete code step + orchws_test step. reclassify I13 MISSING -> DONE(템플릿) / 오진.
  - add step: fix plan text :92 and :164 ("orchws.go:78-101에서 ws 줄을 parseWS로 파싱") -> "parseWS는 입력 [ws:id] 태그용, routeOrch 현재 미연결(b4855b7); R-write ws 필드는 하네스가 파싱 안 함". executor haiku-batch-worker.
  - rejected: none guard in parseWS = dead-code edit, changes tag semantics; revisit only if routeOrch rewired.

## other improvements
- order: resolve 미확정 (D1-D3 above) first -> A1+A2 one edit -> plan text fixes -> C.
- A1+A2 single haiku-coder brief; keep loop/relay wording byte-identical (diff check by haiku-checker).
- C: add haiku-checker grep: haiku-prompter.md has task/paths/limits/return/lang + status/reason head lines; tests/hooks test_k_prompter_missing_lang, test_k2_prompter_all_five_keys_no_row PASS.
- C: go build/vet/test = regression only now (no Go change); keep but mark optional.
- B1: fix scope before audit: list = 8 agent files named in plan; return conflicting lines file:line only, no edits in this plan.
- risk: plan :277-279 summary counts (DONE 12/PARTIAL 3/MISSING 2) change after D2/D3 -> update to DONE 14, PARTIAL 2, MISSING 1 once D1 lands as DONE recount.
- must not change: main-thread-guard.py, agent-model-pin.py, orchws.go, caveman-format.py B_KEYS.
