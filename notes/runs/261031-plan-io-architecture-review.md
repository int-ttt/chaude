# review: 입출력 구조 재설계 제안 (261031)
status: review (opus-architect), 실행 아님
input: 사용자 제안 "사용자 입력 -> (opus 또는 haiku -> 토큰 절약형 번역 -> opus) -> caveman -> haiku; 읽기 = opus -> haiku-scout/researcher -> 번역(줄 제한 없음) -> opus; 쓰기 = opus -> haiku용 최적화 프롬프트 -> haiku(쓰기는 별도 claude code 클라이언트로 맥락 유지, 일회성은 일반 에이전트) -> 변경사항/변경방식만 -> opus"
sources: notes/runs/261030-plan-caveman-relay.md, notes/runs/261030-run-caveman-relay.md, notes/runs/261030-plan-interactive-child-review.md, notes/runs/261031-plan-interactive-child.md, notes/FACTS.md:8-10, notes/DECISIONS.md:46
missing: notes/runs/261030-plan-interactive-child.md 없음; DECISIONS.md:43은 caveman 항목 아님(서브에이전트 파일 차단 규칙)

verdict: 처음부터 재설계는 비권장. 읽기·쓰기 흐름은 현 구조(0.8.x)와 대부분 같고, 비용의 큰 몫은 출력이 아니라 Opus 컨텍스트 생성(캐시 쓰기)임.
evidence: 실측 1회(조건 A) Opus $0.5437 = 캐시 쓰기 43,241토큰 $0.346(64%) + 캐시 읽기 368,727토큰 $0.074(14%) + 출력 6,196토큰 $0.124(23%; 가시 4,783 = 18%, thinking 1,413 = 5%). FACTS.md 단가·1h 쓰기($8/MTok) 기준 계산이 CLI total_cost_usd와 일치.

## 흐름별 판정
입력(사용자 -> 압축 번역 -> Opus): 비권장. 사용자 입력은 보통 1k토큰 미만이라 Opus 입력비 $0.01 미만. 절반 압축해도 Haiku 홉 비용·지연·의도 손실 위험이 더 큼. 원문 그대로 전달, 긴 붙여넣기는 파일 저장 후 scout가 읽음.
출력(Opus caveman -> Haiku 존댓말): 조건부 권장. 가시 출력 50% 감소 가정 시 호출당 약 $0.048(9%) 절감, Haiku 번역 약 $0.003 -> 순이득 있으나 작음. 사용자에게 보이는 최종 답에 1회만 적용, 턴마다 번역 홉 금지. E-ID·확신 태그 유지 규칙 필수.
읽기: 권장(현행과 동일). 단 "줄 제한 없음"은 반대. Opus에 들어가는 1k토큰마다 1h 캐시 쓰기 $0.008 -> 무제한 10k 반환이면 호출비 약 15% 증가, 150k 회전도 앞당김. 기본 상한(4000자) 유지 + Opus가 필요 시 추가 발췌 요청. 별도 번역 홉 없이 scout가 바로 key: value로 반환.
쓰기: 최적화 지시·"변경 파일/방식/검사 결과만 회신"은 권장. 별도 인터랙티브 클라이언트는 비권장. 이유 1: 현 child가 이미 상주 `claude -p --input-format stream-json` 세션이라 맥락 유지됨. 이유 2: 인터랙티브 TUI로 바꾸면 비용·권한·알림 신호 상실(261030-plan-interactive-child-review.md:32). 이유 3: Haiku 콜드스타트는 약 $0.009로 원래 쌈. 이유 4: 맥락 100k 초과 시 Haiku 단가 5배(FACTS.md:10).

## 주요 위험
1. 홉마다 근거·조건부 표현 누락 가능(압축·번역 각 1회).
2. 홉마다 지연 추가, 실패 지점 증가.
3. 장기 상주 Haiku 맥락: 100k 초과 시 5배 단가, 이전 작업 맥락 오염.
4. 회전 상한이 150k로 통일됨(DECISIONS.md:46) -> Haiku child는 100k~150k 구간에서 비싸게 동작.
5. 측정 근거가 A 1회뿐 -> caveman 이득 미확인.

## 권장 구조
입력: 사용자 원문 -> 메인 -> Opus (압축 홉 없음, 대용량은 파일 경로로).
읽기: Opus -> haiku-scout/researcher -> key: value 발췌(기본 4000자, 요청 시 확장) -> Opus.
쓰기: Opus -> 정확한 사양 -> 작업 흐름별 상주 stream-json haiku-coder child(100k 전 회전) / 일회성은 일반 에이전트 -> 변경 파일·방식·검사 결과만 -> Opus.
출력: Opus는 영어 key: value로 작성, 사용자에게 보이는 마지막 답만 메인 Haiku가 존댓말로 변환.
최대 절감 수단: Opus 생성 횟수 축소, TTL 안에서 Opus 재사용, Opus 시스템 프롬프트·도구 정의 축소(캐시 쓰기 43k).

## 도입 전 측정
1. caveman B·C 실행 마무리(A만 완료, $1 상한에서 중단): Opus 가시·thinking 감소율, 번역 비용, 근거 누락 건수, A 2회 흔들림.
2. Opus 호출 5회 이상의 비용 구성(캐시 쓰기/읽기/출력 비율).
3. 같은 내용 한국어 vs 영어 출력 토큰 비율.
4. 읽기 상한 4000자 vs 무제한: Opus 재요청 횟수와 Opus 총비용.
5. 쓰기 상주 child vs 매번 새 생성: 작업당 캐시 쓰기, 종료 시 맥락 크기, 100k 초과 빈도, 오류율.
6. 홉별 지연, 구독 5시간 사용률이 API 단가 비율을 따르는지(billing.md:18 미확인).
