# 검토: 261030-plan-caveman-relay-structure.md
date: 261030
reviewer: opus-architect (검토만, 실행 안 함)
verdict: 보완 후 진행 가능. 핵심 전제(상속 여부)가 확인돼 계획 일부가 불필요해졌고, 비교 기준선·비용 항목·판정 기준이 비어 있음
source_confirmed: https://code.claude.com/docs/en/sub-agents (2026-10-30 확인) - 서브에이전트는 부모 대화 컨텍스트를 기본 상속하지 않음(fork만 상속), 모델은 frontmatter로 지정, Agent 호출의 model 인자는 alias(sonnet/opus/haiku)만 가능

## 개선점 (우선순위 순)

1. q1/v1/v2/5절 갱신 - 상속 질문 종결
- q1_context_inheritance: "확인됨: 대화 이력 상속 없음(fork 제외)"로 바꾸고 출처 URL·확인일 기입
- v2_inheritance_test(마커 실험): 삭제 또는 "선택: 1회 sanity check"로 격하
- 4절 판정의 "상속 확인 시"/"상속 미확인" 분기 삭제. "상속 없음" 분기만 남김
- 5절 미확인 항목에서 "상속 여부" 제거
- q1_scope는 유지하되 답을 기입: 대화 이력은 비상속이지만 서브에이전트는 자체 시스템 프롬프트·에이전트 정의 본문·도구 정의를 매 생성마다 새로 적재함. CLAUDE.md 적재 여부는 DECISIONS.md:28 기록("docs: they load CLAUDE.md otherwise")과 대조해 한 줄로 확정할 것

2. fork 금지 조항 추가 (누락된 위험)
- fork 모드는 부모 대화와 부모 모델(Opus)을 그대로 상속 -> 본 제안의 두 목적(컨텍스트 차단, Haiku 이관)을 동시에 무력화
- 계획에 "subagent_type은 haiku-* 정의 에이전트만, fork 사용 금지" 명시
- 모델 고정 경로 명시: haiku-* frontmatter의 model + .claude/hooks/agent-model-pin.py. Agent 호출의 model 인자는 alias만 가능하므로 버전 고정은 frontmatter 쪽에서만 가능하다고 기록

3. 비교 기준선 교정 - 현재 구조가 이미 본 제안과 거의 같음
- 현 하네스: 메인 스레드 작업 도구 없음(main-thread-guard.py), opus-architect는 tools=Agent만(DECISIONS.md:43), 실작업은 haiku-*로 위임. 즉 "메인 Opus가 명령만 전달 + Haiku 생성"은 이미 opus-architect 패턴
- q3 표에 "현재 구조(기준선)" 열 추가: 차이가 무엇인지(예: 메인 자체가 Opus인지, 메인이 Haiku이고 Opus는 에이전트인지)를 먼저 한 줄로 정의
- v4의 비교 대상 A "메인 Opus 직접 수행"은 현 규칙상 불가능(메인 작업 도구 없음). A를 "현재 구조(메인 -> opus-architect -> haiku-*)"로 바꿀 것
- q4_rule_conflict: "사용자 결정 사항"으로만 두지 말고 기존 결정을 먼저 인용 - DECISIONS.md:42 "[main-model]: Opus main only for design turns; threshold switching only for Haiku main". 이와 충돌하는 부분만 사용자 질문으로 남김

4. 비용 모델 보완 - 빠진 항목
- 결과 중계 비용: Haiku 반환문은 메인 Opus 입력 토큰으로 들어감. "명령만 전달"이어도 반환 크기가 Opus 비용을 좌우함 -> 반환 상한(예: ≤5줄 + 파일 경로) 조건을 계획에 명시
- 브리프 비용: 상속이 없으므로 메인 Opus가 필요한 맥락을 브리프에 직접 써야 함(Opus 출력 토큰 증가). 브리프 길이를 측정 항목에 추가
- 생성당 고정비: 서브에이전트마다 시스템 프롬프트·도구 정의 적재(캐시 쓰기 가능성). billing.md에서 캐시 쓰기/읽기 단가와 서브에이전트 캐시가 부모와 공유되지 않는지 확인 항목으로 유지
- 메인 Opus 자체 이력의 턴당 캐시 읽기 비용도 포함(메인이 오래 살수록 누적)

5. 측정·판정 기준 구체화
- v3: notes/metrics/2026-10-30.jsonl에서 읽을 필드명, 서브에이전트 usage를 agentId로 귀속하는 방법을 명시
- 과제 세트 고정: 최소 3종(조회 1, 코드 수정 1, 분석 1)을 A(현재 구조)/B(제안 구조)에 동일 입력으로 실행
- 판정 임계값 추가: 예) 총비용 B ≤ A의 80% 그리고 5.4 지표 중 결론 일치 100%, 근거 누락 0건이면 채택. 1회 실행은 "방향성만" 기록(현행 유지)
- q2_quality: Haiku 2회 실패 시 sonnet-analyst로 승격하는 현 규칙을 비용에 포함(재시도·승격 비용)

6. 실행 담당·산출물·불변 항목 누락
- 단계별 담당 에이전트 기입: 문서 확인 haiku-researcher, 로컬 grep haiku-scout, jsonl 차분 haiku-runner 또는 haiku-batch-worker, 결과 비교 sonnet-analyst
- 결과 저장 경로 지정: notes/runs/261030-result-caveman-relay-structure.md
- 확인된 사실 반영: notes/FACTS.md에 상속 규칙 한 줄 추가(출처 URL 포함)를 단계로 넣을 것
- 변경 금지 항목 명시: notes/runs/261030-plan-caveman-relay.md, CLAUDE.md, .claude/settings.json, .claude/hooks/*는 평가 중 수정 금지

## 그대로 둬도 되는 부분
- "출처로만 확정, 추정 금지" 원칙(v1)
- caveman-relay 5.1~5.4 지표 재사용
- 1회 실행은 방향성만 기록하는 판정 원칙
