# 계획서: 메인 Opus 명령 전달 + Haiku 에이전트 생성 구조 평가
date: 261030
status: draft (평가 계획. 상속 질문 확인됨, 나머지 실행 전 확인 필요)
related: notes/runs/261030-plan-caveman-relay.md (수정 안 함, 읽기만)

## 1. 제안된 구조
user_proposal_raw: "메인 opus는 다른 컨텍스트 없이 명령만 전달하고 haiku 에이전트를 생성하는 방식은 어떤지 평가; 에이전트 생성 시 부모 모델 컨텍스트를 상속하는지 여부 확인"
flow: 사용자 -> 메인 Opus(명령만 전달, 다른 컨텍스트 없음) -> Haiku 에이전트 생성(작업 수행) -> 메인 Opus 결과 수령 -> 사용자
intent: 메인 Opus 입력·출력 토큰 절감 (메인 Opus 단가가 가장 높음). 실작업은 Haiku로 이관
assumption_1: 확인됨: 기본 비상속, fork만 상속 (출처: https://code.claude.com/docs/en/sub-agents, 2026-10-30 확인)
assumption_2: 명령 메시지만으로 Haiku가 작업을 수행할 수 있음 (미확인)

## 2. 검토 질문
q1_context_inheritance: 확인됨: 기본 비상속, fork만 상속. 서브에이전트는 부모 대화 이력을 상속하지 않음. fork 모드만 부모 대화를 상속함 (출처: https://code.claude.com/docs/en/sub-agents, 2026-10-30 확인)
q1_scope: 대화 이력은 비상속. 서브에이전트는 자체 시스템 프롬프트·에이전트 정의 본문·도구 정의를 매 생성마다 새로 적재함. CLAUDE.md 적재 여부는 notes/DECISIONS.md:28 기록("docs: they load CLAUDE.md otherwise")과 대조해 한 줄로 확정할 것
q2_quality: Haiku가 명령만으로 과제를 수행할 때 근거 누락·결론 차이가 생기는가 (caveman-relay 5.4 지표 재사용 가능)
q2_cost: 메인 Opus 절감분 vs Haiku 에이전트 생성·입력·결과 중계 비용. 세부 항목은 아래 "비용 항목" 참조
q2_cost_cache: 서브에이전트 입력이 캐시 쓰기·읽기 토큰으로 잡히는지 확인 필요 (notes/research/billing.md 단가 확인)
q3_vs_caveman_relay: 차이점 표 (비교 기준선 = 현재 구조)
  - 기준선 정의: 현재 구조 = 메인(Opus) -> opus-architect(tools=Agent만, DECISIONS.md:43) -> haiku-* 실작업. 메인 스레드는 작업 도구 없음(main-thread-guard.py). 본 제안과의 차이는 "메인 Opus가 Haiku를 직접 생성하는지 vs opus-architect 경유인지"
  - 실행 주체: caveman-relay는 메인 Haiku가 압축·번역, Opus는 opus-architect 에이전트 1개. 본 제안은 메인 Opus가 명령 전달, Haiku 에이전트가 작업
  - 출력 스타일: caveman-relay는 Opus 출력을 caveman 압축. 본 제안은 Haiku 출력을 메인 Opus가 중계 (스타일 축 없음)
  - 모델 역할: caveman-relay는 Opus 출력 감소 목적. 본 제안은 Opus 입력·추론 감소 목적
  - 규칙 충돌: CLAUDE.md 4행 "메인 스레드는 작업 도구 없음" 기준으로 메인 Opus가 에이전트를 생성하는 것이 허용되는지 확인 필요. caveman-relay는 메인 Haiku가 직접 수행하고 에이전트는 opus-architect만 호출
  - 품질 위험: 본 제안은 Haiku 단독 작업이라 근거·뉘앙스 손실 위험이 caveman-relay보다 클 수 있음 (미실측)
  - 측정: caveman-relay 5.1~5.3 지표 구조 재사용 가능. 단 Opus 입력 항목이 추가됨
q4_rule_conflict: 기존 결정 먼저 인용: notes/DECISIONS.md:42 "[main-model]: Opus main only for design turns; threshold switching only for Haiku main". 이와 충돌하는 부분("메인 Opus가 명령만 전달하며 작업 생성")만 사용자 질문으로 남김

## 2A. 제약 (필수)
- fork 사용 금지: fork 모드는 부모 대화와 부모 모델(Opus)을 그대로 상속하므로 본 제안의 두 목적(컨텍스트 차단, Haiku 이관)을 무력화함. subagent_type은 haiku-* 정의 에이전트만 사용
- 모델 고정 경로: haiku-* 정의 파일(.claude/agents/haiku-*.md)의 model 항목 + .claude/hooks/agent-model-pin.py 로만 고정. Agent 호출의 model 인자는 alias(sonnet/opus/haiku)만 가능하므로 버전 고정은 frontmatter 쪽에서만 가능

## 3. 검증 방법
v1_inheritance_source: 상속 여부는 출처로만 확정. 추정 금지
  - 1순위: Claude Code 공식 문서(서브에이전트·Task 도구 항목)의 컨텍스트 전달 규칙 인용 (URL·버전·날짜 기록). 확인됨: 기본 비상속, fork만 상속
  - 2순위: 로컬 설치 Claude Code 소스·문서 확인 (설치 경로·버전 기록)
  - 3순위: 저장소 내 기존 노트(notes/research, notes/FACTS.md, .claude/agents/*.md) 확인. 현재 grep 결과 상속 관련 기록 없음 (inherit/상속/fork 검색 0건)
  - 출처 없으면 "상속 여부 미확인"으로 표시하고 결론 보류
v2_inheritance_test: 선택: 1회 sanity check. 메인 세션에 식별 가능한 마커 문구를 넣고, 에이전트에게 마커 존재 여부만 묻기. 결과를 턴 ID와 함께 기록. 판정 근거로는 쓰지 않음
v3_cost_measure: notes/metrics/2026-10-30.jsonl 차분으로 메인 Opus 입력·출력, 에이전트 usage 측정 (caveman-relay 5.2 방법 준용)
  - 읽을 필드: 메인 Opus 입력·출력·캐시 읽기/쓰기 토큰 필드와 서브에이전트 usage 필드 (jsonl 실제 키 이름을 먼저 확인하여 기입)
  - 귀속: 서브에이전트 usage는 agentId 기준으로 메인 턴과 묶어 합산
v4_quality: caveman-relay 5.4 지표(근거 누락, 결론 일치, 뉘앙스, 설명 규칙)로 Haiku 단독 결과를 A(현재 구조: 메인 -> opus-architect -> haiku-*)와 B(제안: 메인 Opus -> haiku-* 직접 생성)와 비교
  - 과제 세트 (최소 3종, A/B에 동일 입력): 조회 1, 코드 수정 1, 분석 1
v5_rule_check: CLAUDE.md 4행과 대조하여 메인 Opus의 에이전트 생성이 허용 범위인지 기록

## 4. 판정
- 상속 없음 확인 (확인됨): 비용·품질·규칙 충돌 3항목으로 판정. 1회 실행이면 방향성만 기록
- 채택 임계값:
  - 총비용 B ≤ A의 80%
  - 5.4 지표 중 결론 일치 100%, 근거 누락 0건
  - 두 조건을 모두 충족하면 채택. 1회 실행은 "방향성만" 기록(현행 유지)
- 출처 미확보 시: 결론 보류, 추가 조사 항목으로 기록

## 비용 항목 (필수 3개)
- 반환문 입력비: Haiku 반환문은 메인 Opus 입력 토큰으로 들어감. 반환 상한(예: ≤5줄 + 파일 경로)을 조건으로 두고 크기를 측정
- 브리프 비용: 상속이 없으므로 메인 Opus가 필요한 맥락을 브리프에 직접 써야 함(Opus 출력 토큰 증가). 브리프 길이를 측정
- 에이전트 고정비: 서브에이전트마다 시스템 프롬프트·도구 정의 적재(캐시 쓰기 가능성). billing.md에서 캐시 쓰기/읽기 단가와 서브에이전트 캐시가 부모와 공유되지 않는지 확인
- 참고: 메인 Opus 자체 이력의 턴당 캐시 읽기 비용도 포함(메인이 오래 살수록 누적). Haiku 실패 2회 시 sonnet-analyst 승격 비용 포함

## 5. 미확인 항목
- 캐시 처리 방식 (billing.md에 항목 확인 필요)
- 메인 Opus 에이전트 생성의 규칙 허용 여부 (사용자 결정 필요)
