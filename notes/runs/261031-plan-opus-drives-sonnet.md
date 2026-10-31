# opus가 sonnet 서브에이전트를 구동하는 설계·오케스트레이션 계획서

status: 계획안 (코드 변경 없음, 실행 전 사용자 확인 필요 항목 §6)
date: 2026-10-31
author: haiku (plan 작성, 파일 1개만 생성)
input (읽기만):
- notes/runs/261031-answer-usage-distribution.md
- notes/runs/261031-answer-weekly-usage.md
- notes/runs/261031-plan-caveman-c.md (202행 중 1-154, 155-202 확인)
- notes/runs/261031-plan-caveman-c-review.md
- .claude/agents/sonnet-analyst.md, sonnet-reviewer.md, haiku-scout.md
- .claude/hooks/agent-model-pin.py
scope_note: 이 계획은 설계·위임 계획까지다. 릴리즈, 태그, 버전 번호, CHANGELOG 릴리즈 항목은 범위 밖이다 (§7).

## 1. 요청 원문과 해석

- 요청 원문: "sonnet-analyst, sonnet-reviewer를 opus가 서브에이전트로 구동하도록 변경; scout 아키텍처 개선"
- 해석 1: 'socut'은 'scout'의 오타로 본다. 따라서 두 번째 항목은 "scout 아키텍처 개선"이다. 대상은 .claude/agents/haiku-scout.md와 그 호출 방식이다.
- 해석 2: "opus가 서브에이전트로 구동"은 opus-architect(서브에이전트)가 sonnet-analyst, sonnet-reviewer를 호출한다는 뜻으로 본다. 메인 스레드에서 호출하는 것과 구분한다.
- 해석 3 (미확정): 현재 메인 스레드의 호출 체계와 어떻게 공존할지는 §6 질문에서 확인한다.

## 2. 현재 제약 (파일 근거)

agent-model-pin.py 규칙은 다음과 같다.

| 규칙 | 근거 | 내용 |
|---|---|---|
| 서브에이전트 호출자 제한 | agent-model-pin.py:27 | SUB_CALLERS = {sonnet-reviewer, sonnet-analyst, opus-architect} |
| 서브에이전트는 haiku-*만 호출 | agent-model-pin.py:45-50 | agent_id가 있고 호출자가 SUB_CALLERS이면 target이 haiku-*가 아니면 deny |
| 최종 방어선 | agent-model-pin.py:75-79 | agent_id가 있고 kind가 haiku-*가 아니면 deny |
| 메인 스레드 호출 | agent-model-pin.py:87-96 | agent_id가 없는 메인 호출은 model 인자만 제거하고 허용 |
| E gate (CHAUDE_ORCH=e) | agent-model-pin.py:53-59 | 메인 스레드가 sonnet-/opus-를 호출하려면 prompt에 ESCALATE가 있어야 함 |
| E gate RETURN 상한 | agent-model-pin.py:61-72 | RETURN 상한이 원본 크기의 0.25배 초과이거나 없으면 deny |
| 문서 근거 | agent-model-pin.py:8-10 | docstring: v0.8.2에서 sonnet-reviewer도 서브에이전트 대상에서 제외됨 |

- 에이전트 파일 근거: sonnet-analyst.md:4 (tools: Agent)과 :18 (haiku-* 위임 전제). sonnet-reviewer.md:4, :11 (haiku-scout, haiku-checker 위임 전제).
- 결론: "opus-architect(서브에이전트)가 sonnet-analyst를 구동"하는 것은 agent-model-pin.py:45-50에서 차단된다. 이 변경은 훅과 충돌한다.
- 충돌하지 않는 구조: 메인 스레드가 sonnet-analyst를 직접 호출하는 것은 :87-96에서 허용된다. 다만 CHAUDE_ORCH=e 모드이면 E gate(:57-59)에 ESCALATE 문구가 필요하다.

## 3. 대안 비교: opus가 sonnet을 구동하는 문제 해결

| 안 | 내용 | 장점 | 단점·위험 | 변경 파일 |
|---|---|---|---|---|
| A. 훅 예외 추가 | agent-model-pin.py:45-50에 opus-architect -> sonnet-analyst/sonnet-reviewer 허용 추가 | 요청 문구 그대로 구현 | v0.8.2 정책(sonnet/opus 서브에이전트는 haiku-*만)을 되돌림. 재귀 방지 목적 훼손. 훅 테스트 추가 필요 | agent-model-pin.py:27, 40-50, 8-11 docstring, tests/hooks/test_orch_gate.py |
| B. 구동 주체 재배치 | sonnet-analyst·sonnet-reviewer를 opus-architect의 하위가 아니라 메인 스레드(또는 메인 Opus 세션)가 직접 호출. opus-architect는 설계안(plan 파일)만 생성 | 훅 변경 없음 (:87-96 허용). v0.8.2 정책 유지 | "opus가 구동"이 아니라 "메인이 구동"이 됨. 메인 컨텍스트가 sonnet 결과를 받아 커지므로 caveman R-decide 반환 규칙 필요 | CLAUDE.md 라우팅 문구, haiku-prompter.md:12 LOOP_TARGETS |
| C. sonnet 역할을 haiku로 대체 | sonnet-analyst 역할은 opus-architect 단독 또는 haiku-batch-worker·haiku-drafter로, sonnet-reviewer 역할은 haiku-checker로 대체 | 훅 변경 없음. 서브에이전트 체인 깊이 2 유지. 이전 계획 선택지 C와 일치 | sonnet 판단 품질 손실 가능. 판정 주체가 opus에 몰려 비용 증가 가능 (budget.go:92-94 단가 가정, 공식 단가 미검증) | sonnet-analyst.md, sonnet-reviewer.md 삭제 또는 보존, CLAUDE.md, orch.go:38-44 |

- 권장 순서: 안 B를 먼저 검토하고, 요청 문구를 꼭 지켜야 하면 안 A를 선택한다. 안 A는 v0.8.2 정책을 바꾸므로 사용자 승인이 필요하다 (§6 Q1).
- 안 C는 이전 계획(261031-plan-caveman-c.md §1)의 방향과 같다. 요청과 다르므로 사용자가 방향을 확정해야 한다.

## 4. scout 아키텍처 개선안

### 4.1 근거 사실과 확인 상태

- 요청자가 제시한 수치: scout 536회 호출, 호출당 캐시 재읽기 약 70%, 컨텍스트 100K 초과 0건.
- 저장소 확인 결과: notes/runs, notes/FACTS.md, notes/LEDGER.md에서 "536"과 scout 호출 집계를 찾지 못했다. 캐시 재읽기 70%와 100K 초과 0건도 확인되지 않았다. 이 수치는 미확인으로 두고 개선 근거로 쓰지 않는다.
- 저장소에 있는 근거:
  - 10-29~31 스폰 집계 (261031-answer-usage-distribution.md:4): haiku-scout 20건 (스폰 32건 중 62.5%). 8 turn 기준.
  - 10-28~31 턴 집계 (261031-answer-weekly-usage.md:20): haiku-scout 22건 (스폰 37건 중 59.5%). spawn_types 기준. 같은 파일 :25 기준 agent 키 레코드에서는 haiku-scout 25건.
  - 두 집계의 기간과 기준이 달라 합산하지 않는다. 누락 가능성도 있다 (weekly-usage :43).
  - 캐시 합계 (usage-distribution:18): 8개 spawn turn 합계 cache_read 478,348. 에이전트별 분리는 불가.
  - 에이전트별 토큰은 기록되지 않았다 (weekly-usage:37).
- 결론: scout 호출 수는 10-31 기준 20~25건 수준으로 집계된다. 536회는 다른 기간이거나 다른 기준일 가능성이 있다. 캐시 재읽기 비율은 측정되지 않았다.

### 4.2 개선 후보 (측정 전 가설)

| 후보 | 내용 | 기대 효과 (가설) | 검증 방법 |
|---|---|---|---|
| S1. 다중 경로 묶음 호출 | haiku-scout 1회 호출에 관련 파일을 최대 3개까지 묶음 (sonnet-analyst.md:18의 "최대 3개 파일" 규칙을 scout에도 적용) | 호출 수 감소 | 같은 주제 호출 수를 metrics spawn_types로 전후 비교 |
| S2. 결과 재사용 | scout 결과를 notes/runs/<YYMMDD>-scout-<topic>.md에 저장하고, 같은 경로를 다시 읽지 않도록 brief에 기존 결과 경로를 넣음 | 재읽기 감소 | 동일 경로 반복 read 횟수를 transcript 없이 확인하기 어려우므로 brief 로그 기준 |
| S3. 전수 검색을 haiku-checker로 이관 | grep 전수 목록은 scout, 테스트 실행은 checker (caveman-c-review.md 항목 29) | 중복 실행 감소 | P0 단계 중복 실행 건수 |
| S4. effort 조정 | haiku-scout.md:6 effort high를 단순 조회 기준 medium으로 낮춤 | 토큰 감소 가능 | 정확도 저하 여부를 샘플 10건으로 확인 |
| S5. autoCompactWindow | haiku-scout.md:8 autoCompactWindow 100000 유지 여부 확인 | 컨텍스트 상한 관리 | 100K 초과 건수를 metrics에서 확인 (현재 미확인) |
| S6. 메인 직접 Grep | 한 파일 확인은 scout 대신 메인 Grep으로 처리하도록 haiku-scout 호출 기준 명시 | 호출 수 감소 | 한 파일 호출 건수 집계 |

- 우선순위 제안: S1과 S2를 먼저 적용하고, S4와 S5는 측정 후 결정한다.
- 금지: 측정 없이 "70% 절감" 같은 수치를 문서에 쓰지 않는다.

## 5. 실행 단계 (Haiku 에이전트별 위임)

원칙: 단계마다 커밋. 작업 브랜치는 master에서 분기. master 작업 트리 직접 수정 금지. stash 금지. 차단형 훅 추가 금지. 훅 변경은 §6 Q1 승인 후에만.

| 단계 | 위임 에이전트 | 위임 항목 | 반환 형식 | 검증 명령 |
|---|---|---|---|---|
| P0 기준선 | haiku-scout | 10-28~31 metrics에서 haiku-scout 스폰 수, 에이전트별 턴 수, cache_read 합계를 집계. 536·70%·100K 수치의 출처 파일 검색 | R-read | `ls notes/metrics/` 와 `grep -rn "536" notes/runs notes/LEDGER.md` |
| P0 기준선 | haiku-checker | 집계 스크립트 결과 재계산 (python3 -I 사용) | R-write (check) | 같은 입력으로 두 번 실행해 결과 일치 확인 |
| P1 scout 규칙 문서 | haiku-batch-worker | S1·S2 규칙을 haiku-scout.md 본문에 추가 (≤20줄). 변경 전 원본을 notes/archive/261031/ 아래 복사 | R-write | `git diff -- .claude/agents/haiku-scout.md` |
| P1 검증 | haiku-checker | 파일 형식 확인, frontmatter 유지 확인 | R-write (check) | `head -9 .claude/agents/haiku-scout.md` |
| P2 호출 기준 | haiku-coder | opus-architect.md에 "haiku 병렬 호출 ≤4개/과제, 반환 각 ≤4000자, 응답 ≤20회" 상한 추가 (caveman-c-review 항목 16) | R-write | `grep -n "4000" .claude/agents/opus-architect.md` |
| P3 호출 구조 (안 B 또는 C 선택 후) | haiku-coder | 선택한 안에 맞춰 sonnet-analyst.md, sonnet-reviewer.md, CLAUDE.md 라우팅 문구 수정. 안 A이면 agent-model-pin.py:27, 40-50 수정 | R-write | 안 A: `python3 tests/hooks/test_orch_gate.py`. 안 B·C: `grep -rn "sonnet-analyst\|sonnet-reviewer" .claude CLAUDE.md` 결과 목록 |
| P3 훅 테스트 | haiku-checker | .claude/hooks 테스트 실행 (러너: `python3 -m unittest discover -s .claude/hooks/tests -p 'test_*.py'`) | R-write (check) | 실패 0 |
| P4 scout 측정 | haiku-batch-worker | P1 적용 후 같은 기간 spawn 수를 다시 집계하고 P0 기준선과 비교표 작성 | R-write | 집계 입력 파일 수와 행 수를 함께 기록 |
| P5 정합 확인 | haiku-scout | 저장소 전체에서 sonnet-analyst, sonnet-reviewer 참조를 재grep (exclude .git, logs, notes/archive, notes/runs, notes/metrics) 하고 허용·불허 분류 | R-read | `grep -rn "sonnet-analyst\|sonnet-reviewer" --exclude-dir=.git --exclude-dir=logs --exclude-dir=notes .` |
| P6 최종 검증 | haiku-checker | 훅 테스트, tests/hooks/test_orch_gate.py, `go test ./...`(Go 변경이 있을 때만) 실행 | R-write (check) | 실패 0 |

- opus-architect는 실행 단계에 직접 투입하지 않는다. 재설계가 필요하면 opus-architect 호출 1회만 허용하고 ESCALATE 사유를 기록한다.
- sonnet 호출은 안 A가 승인될 때만 허용한다. 그 외에는 없다.
- 검증 실패 시: 해당 단계 커밋 전 중단, haiku-coder 재시도 1회, 그래도 실패하면 status: fail과 reason을 기록하고 사용자에게 보고한다.

## 6. 사용자 확인이 필요한 질문

- Q1. 안 A(훅 예외 추가, v0.8.2 정책 되돌림), 안 B(구동 주체를 메인으로 재배치), 안 C(sonnet 역할을 haiku로 대체) 중 무엇을 택하는가?
  - 선택지: (a) A (b) B (c) C (d) 보류
- Q2. "opus가 서브에이전트로 구동"을 꼭 지켜야 하는가, 아니면 목적(판단 품질 확보)만 지키면 되는가?
- Q3. "536회" 수치의 출처와 기간은 무엇인가? 저장소에서 찾을 수 없었다. 기준 기간과 집계 방법을 알려 달라.
- Q4. "캐시 재읽기 약 70%"와 "컨텍스트 100K 초과 0건"은 어디서 측정했는가? 저장소에서 확인되지 않았다.
- Q5. scout 개선의 우선순위는 호출 수 감소(S1, S6)인가, 토큰 감소(S4, S5)인가?
- Q6. 훅을 바꾸는 경우 E gate(agent-model-pin.py:53-59)를 유지하는가, 예외를 두는가?
- Q7. sonnet-analyst.md와 sonnet-reviewer.md를 삭제하는가, 보존하고 호출만 막는가?
- Q8. 기존 사용량 집계 두 건의 차이 (scout 20건 vs 22건, 기간 10-29~31 vs 10-28~31)를 어느 쪽 기준으로 볼지 정하는가?

## 7. 범위 밖 (명시)

- 릴리즈 생성, 재릴리즈, 버전 번호 변경, 태그 생성 또는 삭제, CHANGELOG 릴리즈 항목 작성. 이 계획은 다루지 않는다.
- 호환성 영향 (안 A·B·C 공통): sonnet-* 에이전트 호출 경로가 바뀌므로 사용자 안내 문서 갱신이 필요하다. 릴리즈 노트 처리는 별도 계획에서 다룬다 (notes/runs/261031-plan-release-v082-v090.md 참조).
- 비용 실측과 단가 확정은 이 계획 범위가 아니다 (U1=a, 구현 후 측정).

## 8. 미확인 목록

- 536회, 70%, 100K 초과 0건의 출처 (§4.1)
- 에이전트별 토큰·비용·시간 (weekly-usage:37)
- sonnet-researcher 사용 기록 (usage-distribution: 0건으로 보고됨)
- agent-model-pin.py의 서브에이전트 호출 차단 동작은 훅 코드 읽기로 확인했고, 실제 실행 로그는 확인하지 않았다.
