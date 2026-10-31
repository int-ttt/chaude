# 261031-plan-opus-drives-sonnet 검토: 개선안

target: notes/runs/261031-plan-opus-drives-sonnet.md (124행)
reviewer: opus-architect (읽기는 haiku-scout 위임, 실행 안 함)
evidence: .claude/hooks/agent-model-pin.py, notes/runs/261031-answer-usage-distribution.md, notes/runs/261031-answer-weekly-usage.md, notes/runs/261031-plan-caveman-c-review.md, notes/DECISIONS.md, notes/LESSONS.md
format: 번호. 근거 경로:행 | 수정 제안 한 줄
critical: 1, 11, 18

## (a) 훅 충돌과 해결안

1. plan:44 / agent-model-pin.py:75-79 | 안 A 변경 범위에 backstop(:75-79, agent_id 있으면 haiku-* 외 deny) 누락. :45-50만 고치면 :75-79에서 다시 차단되므로 변경 파일을 :27, :45-50, :75-79, docstring :8-10 네 곳으로 고친다.
2. plan:31 / agent-model-pin.py:86-91, :95-96 | 메인 스레드 근거 행 ":87-96"을 "model 인자 제거 :86-91, 그 외 통과 :95-96"으로 정정한다.
3. plan:44 / agent-model-pin.py:27, :45-50 | 안 A 채택 시 예외를 "caller=opus-architect, target∈{sonnet-analyst, sonnet-reviewer}"로만 한정하고 sonnet-* caller는 haiku-*만 유지해 체인 깊이를 main→opus→sonnet→haiku 3단으로 묶는다는 문장을 넣는다.
4. plan:44 / tests/hooks/test_orch_gate.py | 안 A 테스트 케이스를 명시한다: allow opus-architect→sonnet-analyst, allow opus-architect→sonnet-reviewer, deny sonnet-analyst→sonnet-reviewer, deny opus-architect→opus-architect, deny opus-architect→haiku-prompter(:33-38).
5. plan:34 / notes/DECISIONS.md:35, :43, notes/LESSONS.md:5 | 서브에이전트의 sonnet-reviewer 호출은 261029에 허용(:35)했다가 v0.8.2(:43)에서 다시 막은 정책이다. §2에 세 근거를 인용하고, 닫은 이유를 확인한 뒤 안 A를 평가한다(LESSONS 실패 반복 금지).
6. plan:45 / agent-model-pin.py:39-44, .claude/agents/haiku-prompter.md:12 | 안 B'를 추가한다: opus-architect가 plan에 `## [sonnet] steps`(대상 agent, brief 전문, return 상한)를 쓰고 메인 또는 haiku-prompter(:39-44에서 sonnet-* 호출 이미 허용)가 그대로 전달한다. 훅 변경 없이 "opus가 지시"를 충족한다.
7. plan:45 / haiku-prompter.md:12 | LOOP_TARGETS에 sonnet-analyst, sonnet-reviewer가 이미 있으므로 안 B 변경 파일에서 haiku-prompter.md:12를 빼거나 "변경 불요"로 정정한다.
8. plan:46 | 안 C의 sonnet-reviewer→haiku-checker 대체는 반출 전 검토를 단순 검사로 낮춘다. "reviewer 보존, analyst만 대체"인 안 C'를 별도 행으로 둔다.

## (b) 대안 비용·품질·위험 비교

9. plan:46 / answer-weekly-usage.md:36, answer-usage-distribution.md:6 | 비용 근거를 budget.go 단가 가정 대신 실측 비중으로 바꾼다: sonnet 0.63%(턴 cost 합) 또는 5.9%(세션 누적 파생), opus 1.23% 또는 6.3%. 두 기준 값이 달라 어느 기준인지 먼저 확정한다는 문장을 붙인다.
10. plan:42-46 / answer-weekly-usage.md:36, answer-usage-distribution.md:5-6 | haiku가 비용의 87.9~98.1%이므로 A/B/C 선택의 비용 효과는 작다. "이 결정의 목적은 품질·구조이고 비용 절감이 아니다"를 §3에 쓰고 표에서 비용 열과 품질·위험 열을 분리한다.
11. plan:44-45 / plan-caveman-c-review.md:28 | A와 B 모두 opus 또는 메인이 sonnet 반환을 받으므로 반환 상한(R-decide, 각 ≤4000자, 호출 수 상한)을 두 안 공통 조건으로 표에 넣는다. 없으면 상위 컨텍스트가 커진다.

## (c) scout 아키텍처 개선안 근거

12. plan:85, :92 / answer-weekly-usage.md:37, answer-usage-distribution.md:14 | 에이전트별 토큰 기록이 없다(sub_* 12건, 에이전트 구분 없음). P0의 에이전트별 cache_read는 산출 불가이므로 P0a(sub_* 레코드에 agent_type 기록 계측)를 추가하거나 P0·P4 지표를 spawn 수로 한정한다.
13. plan:69 / answer-usage-distribution.md:17 | S1 근거로 "10-31 07:53:59Z 한 turn에 haiku-scout 6회 스폰"을 적고 효과 지표를 "turn당 scout 스폰 수"로 정한다.
14. plan:72-73 | S4(effort)·S5(autoCompactWindow)는 근거 데이터가 없다. P0a 계측 전까지 "보류"로 표시한다.
15. plan:70 | S2 결과 재사용은 오래된 내용을 다시 쓸 위험이 있다. 저장 파일에 경로별 git hash 또는 mtime을 기록하고 일치할 때만 재사용하는 조건을 넣는다.
16. plan:74 / agent-model-pin.py:61-72, notes/DECISIONS.md:43 | S6 "메인 직접 Grep"은 메인 스레드에만 해당한다(opus/sonnet 서브에이전트는 tools=Agent only). E 모드는 :61-72가 이미 inline을 유도하므로 적용 대상을 메인으로 한정하고 CLAUDE.md 메인 규칙과 충돌 확인 단계를 넣는다.
17. plan:69 / 본 검토 세션 관찰 | R-read 상한 4000자 때문에 6252자(wc -m) 계획서 하나를 읽는 데 scout 4회가 들었고 그중 2회가 truncated였다. S7 후보 "검토 대상 단일 파일이 상한의 2배 이하이면 전문 반환 허용"을 추가한다.
18. plan:63 / answer-usage-distribution.md:4, answer-weekly-usage.md:28 | "10-31 기준 20~25건"을 정정한다: 20건은 10-29~31 spawn_types 기준, 25건은 10-31 agent 키 기준. 기간과 기준을 각각 붙인다.

## (d) 실행 순서와 검증 명령

19. plan:92 | P4 "같은 기간 재집계"는 변경 전 데이터라 비교가 무의미하다. "P1 적용 후 새 기간 N일(사용자 확인)을 기준선과 같은 길이로 집계"로 바꾼다.
20. plan:93 | 설명은 notes/archive·runs·metrics만 제외인데 명령은 --exclude-dir=notes로 notes 전체를 뺀다(DECISIONS.md·FACTS.md 누락). 명령을 `grep -rn -E "sonnet-analyst|sonnet-reviewer" . --exclude-dir=.git --exclude-dir=logs --exclude-dir=archive --exclude-dir=runs --exclude-dir=metrics`로 바꾼다.
21. plan:91, :94 | 훅 테스트가 두 곳에 있다(.claude/hooks/tests/test_caveman_format.py, tests/hooks/test_orch_gate.py). P3·P6 명령을 `python3 -m unittest discover -s .claude/hooks/tests -p 'test_*.py'`와 `python3 -m unittest discover -s tests/hooks -p 'test_*.py'` 둘 다로 적는다.
22. plan:90 | 안 B·C 검증이 grep 목록 출력뿐이라 합격 기준이 없다. P3 전에 허용 참조 목록(파일:행)을 확정하고 "목록 외 참조 0건"을 합격 기준으로 둔다.
23. plan:83-94 | 표에 "Q1 답변 → P3" 게이트를 적고, Q1과 무관한 P2(opus-architect.md 상한)는 P1과 병렬 가능으로 표시한다.
24. plan:83-94 / notes/DECISIONS.md:43, notes/LESSONS.md:5 | P3 뒤에 P3b를 추가한다: haiku-batch-worker가 결정을 DECISIONS.md에 기록하고, 안 A면 LESSONS.md:5를 갱신한다.
25. plan:87 / plan-caveman-c-review.md:21 | notes/archive/261031/ 원본 복사는 작업 브랜치로 대체한다. archive 사본이 문제를 일으킨 전례(:21)가 있다.
26. plan:85 | `grep -rn "536"`은 관계없는 숫자까지 잡는다. `grep -rnE "536 ?(회|calls|spawn)" notes/`처럼 단위가 붙은 패턴으로 바꾼다.

## (e) 사용자 확인 질문

27. plan:108 / agent-model-pin.py:45-60 | Q6를 고친다: E gate는 메인 스레드(agent_id 없음)에만 걸려 안 A의 opus→sonnet 호출에는 원래 적용되지 않는다. "opus→sonnet 호출에도 ESCALATE 문구를 요구할지"로 묻는다.
28. plan:110 / answer-usage-distribution.md:4, answer-weekly-usage.md:28 | Q8 수치를 고친다: "20건 vs 22건" → "20건(10-29~31, spawn_types) vs 25건(10-31, agent 키)". sonnet-analyst 6건 vs 7건, sonnet 비용 0.63% vs 5.9% 기준 선택도 함께 묻는다.
29. plan:100-110 | Q9를 추가한다: 현재 CHAUDE_ORCH 모드가 e인가. 안 B에서 메인이 ESCALATE를 넣어야 하는지가 여기서 갈린다.
30. plan:100-110 | Q10을 추가한다: 안 A면 예외를 opus-architect caller로만 한정하는가, sonnet-analyst→sonnet-reviewer도 허용하는가.
31. plan:100-110 | Q11을 추가한다: 메인이 Haiku 메인일 때 안 B(메인이 sonnet 직접 호출)로 충분한가, 안 B'(opus가 쓴 brief를 메인·haiku-prompter가 전달)를 원하는가. 그리고 P0a 계측(metrics에 agent_type 추가) 승인과 P4 측정 기간 N일을 함께 묻는다.
