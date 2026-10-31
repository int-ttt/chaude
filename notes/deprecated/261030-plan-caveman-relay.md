# 계획서: caveman 릴레이 (Haiku 압축 전달 -> Opus caveman 출력 -> Haiku 번역)
date: 261030
status: merged (3단계 병합 완료, 검토 16건 반영. 실행 전 임계값·과제 확정 필요)
review: notes/runs/261030-plan-caveman-relay-review.md

## 1. 목적
purpose: Opus 출력 토큰 절감 가능성 판단. 사용자 제안 구조의 타당성·트레이드오프 평가 + 테스트 1회 실행
user_proposal: 메인 Haiku가 Opus에 맥락 최소 토큰 전달 -> Opus caveman 스타일(조사·군더더기 생략) 출력 -> Haiku가 한국어 존댓말로 번역해 전달
constraint: 압축·번역은 메인 Haiku가 직접 수행 (에이전트 호출 아님). 에이전트 호출은 opus-architect(조건 A·B·C의 Opus 출력)만 사용
constraint_source: CLAUDE.md 4행 (메인 스레드는 작업 도구 없음, 토큰 최소화)

## 2. 구조
flow: 사용자 -> 메인 Haiku(맥락 압축) -> opus-architect(caveman 출력) -> 메인 Haiku(존댓말 번역) -> 사용자
compress_rule: 입력 맥락은 사실·제약·질문만, 수식어 제거. 근거 파일 경로는 유지
caveman_rule: Opus 출력은 조사·접속어·인사 생략, key: value / 명사구 위주. 근거(파일·수치) 누락 금지, 불확실 표시 유지
caveman_format (고정 스키마, 9절 근거):
- 근거 행: `E<n>: <경로>:<행> / <수치 또는 인용 요지>` (E1, E2 ... 순번, 1근거 1행)
- 주장 행: `<주장> [확실|추정|불확실] [근거: E<n>,...]`
- 조건문(~라면, ~일 수 있음)은 주장 행 안에 그대로 유지
translate_rule: 메인 Haiku가 caveman 출력을 한국어 존댓말 문장으로 복원. 내용 추가 금지(번역만), 새 사실 창작 금지. 모든 E-ID와 확신 태그를 번역본에 유지
post_check: 번역 후 haiku-checker가 문자열 대조 (9절)

## 3. 조건 정의
- condition_A: 일반 Opus (opus-architect, 기본 스타일). 원 입력 그대로. 메인 Haiku가 결과를 사용자에게 요약·중계 (중계 토큰 측정 대상)
- condition_B: 메인 Haiku 압축 -> opus-architect caveman -> 메인 Haiku 번역 (전체 릴레이)
- condition_C: 원 입력 그대로 -> opus-architect caveman -> 메인 Haiku 번역 (압축 단계 없음)
- 귀속 규칙:
  - C vs A 차이 = caveman 출력 + 번역 효과
  - B vs C 차이 = 압축 단계 효과
  - 근거 누락·결론 차이가 생기면 B vs C 차이가 크면 압축 탓, C vs A 차이가 크면 caveman·번역 탓으로 기록
- 조건 C는 A·B와 같은 세션 조건 규칙 (10절)

## 4. 트레이드오프
gain: Opus 출력 토큰 절감 (Opus 출력 단가가 가장 높음). thinking 토큰은 caveman으로 줄지 않을 수 있어 분리 측정 (5절)
cost: 번역 토큰 추가(Haiku 입력+출력), 압축 단계 토큰, 지연 증가
risk_1: 근거 누락 (압축·caveman·번역 과정에서 증거/출처 탈락)
risk_2: 뉘앙스 손실 (불확실성·조건부 표현 약화, 결론 단정화)
risk_3: 설명 규칙 위반 (사용자 대상 설명 규칙을 번역 단계가 못 지킬 가능성). 체크리스트는 8절
risk_4: 번역 오역/환각 (Haiku가 생략된 조사 복원 시 의미 오해)
risk_5: caveman 스타일 미준수 (Opus가 지시를 일부만 따름). 14절 준수 지표로 별도 판정
limit: 1회 실행이라 통계적 결론 불가. 과제 1개에 한정, 일반화 금지. 결과는 방향성 참고용. A 2회 실행으로 흔들림 기준선 1개만 확보
uncertainty: 순이득 여부는 Opus 출력 감소분(단가 가중) vs 번역+압축+중계 차이 비용으로 결정되며 사전 단정 불가

## 5. 측정 지표 (단위 정의 포함)
### 5.1 토큰 (Opus, 조건별)
- opus_thinking_out: Opus 추론(thinking) 출력 토큰
- opus_visible_out: Opus 가시 출력 토큰 (thinking 제외)
- opus_in_normal: Opus 일반 입력 토큰
- opus_in_cache_read: Opus 캐시 읽기 토큰
- opus_in_cache_write: Opus 캐시 쓰기 토큰
- 출처: 에이전트 결과 usage 필드 그대로 인용. 추정 시 추정 표시

### 5.2 메인 Haiku 토큰 (조건별)
- compress_tok: 압축 단계 Haiku 입력+출력 (조건 B만)
- translate_tok: 번역 단계 Haiku 입력+출력 (조건 B·C)
- relay_tok: 조건 A에서 메인 Haiku가 결과를 중계한 입력+출력
- 측정 방법: 해당 단계 직전·직후 notes/metrics/2026-10-30.jsonl 누적치 차분. 턴 ID 기록. 차분이 다른 턴과 섞이면 "측정 불가"로 표시
- 순비용 정의: 조건 B 순 Haiku 비용 = compress_tok + translate_tok - relay_tok (A 기준). 조건 C 순 Haiku 비용 = translate_tok - relay_tok

### 5.3 단가 가중 총비용
- formula: 총비용 = Σ(모델별 입력 단가 x 일반 입력 + 캐시 읽기 단가 x cache_read + 캐시 쓰기 단가 x cache_write + 출력 단가 x (thinking + 가시 출력))
- 단가 출처: notes/research/billing.md. 실행 전 해당 파일에 모델별 4개 단가(입력·캐시 읽기·캐시 쓰기·출력)가 있는지 확인. 없으면 "단가 미확인"으로 표시하고 토큰만 비교
- 순이득 = (A 총비용) - (B 총비용), B는 5.2 순비용 포함

### 5.4 품질 지표
- 근거 항목 단위: 파일 경로, 행 번호, 수치, 명명된 출처, 인용 1개 = 1항목
- 근거 누락 수: A에서 먼저 추출한 근거 목록 중 최종 번역본에 없는 항목 수 (A 추출은 B를 보기 전에 실행)
- 역방향 근거 수: 번역본에만 있는 근거 항목 수 (추가 근거 여부 표시)
- 결론 일치 대상: 최종 권고 문장 1개 (결론이 여러 개면 "최종 권고" 문장 1개만 비교)
- 결론 일치: 일치 / 부분 / 불일치 (루브릭 판정)
- 뉘앙스 손실: 불확실 표현 유지 수 / A의 불확실 표현 수
- 환각·오역: 있음/없음, 항목
- 설명 규칙 위반 건수: 8절 체크리스트 기준

### 5.5 압축·스타일 지표
- 압축 누락: 압축 입력과 원 입력을 대조해 빠진 사실·제약·경로 수 (haiku-checker)
- 스타일 준수율: caveman 출력 중 `key: value` 또는 명사구 행 비율, 인사·접속어로 시작하는 문장 수. 14절 기준 미달이면 "스타일 미적용"

## 6. 판정 기준 (실행 전 확정)
- 유망: Opus 총 출력(thinking + 가시) 절감 >= 30%, 근거 누락 0건, 결론 불일치 0건, 스타일 준수 통과
- 판단 불가: |A-B| 차이 <= |A1-A2| 차이 (A 2회 흔들림 기준선) 이거나 단가 미확인
- 기각: 순비용(5.3) 기준 B가 A보다 비싸거나, 근거 누락 1건 이상
- 임계값 30%은 계획 초안 값. 실행 전 변경하면 변경 사유를 결과 파일에 기록

## 7. 테스트 설계
task: 과제 1개 고정 (실행 전 확정). 조건:
- 정답 확인 가능한 사실(파일 내용, 수치)을 포함할 것
- notes/ 기존 노트에 답이 정리되어 있지 않을 것 (복사만 하면 차이 안 드러남)
- 설명·분석형 중간 길이 (출력 수백 토큰 이상)
- 두 조건 동일 프롬프트·동일 입력 파일
run_plan: A1, A2 (흔들림 기준선) -> B -> C. 각 조건은 실행 전 원문을 notes/runs/에 저장
order: 조건별 새 세션 또는 캐시 만료 후 실행 (13절)

## 8. 판정 방식
judge: sonnet-reviewer, X/Y 무작위 라벨 (9절 블라인드)
rubric:
- 결론 일치 (5.4 대상 문장 기준: 일치/부분/불일치)
- 근거 누락 수 (항목 열거) + 역방향 근거 수
- 뉘앙스 손실 (불확실 표현 유지 여부)
- 설명 규칙 준수 (위반 건수)
- 환각/오역 (있음/없음, 항목)
설명 규칙 체크리스트 (CLAUDE.md 4행 기준. 이 행 외 사용자 대상 설명 규칙 문서는 확인되지 않음):
1. 한국어로 작성
2. 존댓말 사용
3. 메인 스레드 토큰 최소화 (중계 요약은 필요한 범위만)
위반 건수 = 위 항목 위반 수. 체크리스트 밖 항목은 판정하지 않음.

## 9. 번역 검사와 저장
- 중간 저장: 원 입력, 압축 입력(B), caveman 원문(A 제외), 번역본을 모두 notes/runs/에 저장
- 압축 검사: 압축 입력 vs 원 입력 대조로 빠진 사실·제약·경로 목록 기록 (5.5)
- 번역 검사: haiku-checker가 caveman 원문의 E-ID, 파일 경로, 수치, 확신 태그가 번역본에 모두 문자열로 있는지 대조. 누락 목록을 결과에 기록
- 판정 블라인드: sonnet-reviewer에게 최종본을 X/Y 라벨로 제시. 조건 매핑은 판정 후 공개

## 10. 실행 단계
1. 계획 작성: 본 문서 (sonnet-analyst -> haiku-batch-worker 저장)
2. opus-architect 검토: 구조·위험·테스트 설계 검토 -> notes/runs/261030-plan-caveman-relay-review.md
3. 병합: 검토 지적을 본 계획서에 반영 (완료). 과제 1개 확정 및 임계값 확정은 실행 직전
4. 실행:
   - 메인 Haiku가 직접 압축·번역 (에이전트 호출 금지)
   - opus-architect만 에이전트로 호출 (A1, A2, B·C의 caveman 출력)
   - 각 단계 턴 ID와 metrics 차분 기록
   - 조건 간 캐시 영향: 각 조건을 새 세션 또는 캐시 만료 후 실행. 실행 순서, cache_read 수치를 함께 기록
   - haiku-checker 번역 검사 후 sonnet-reviewer 블라인드 판정
   - 결과는 notes/runs/261030-result-caveman-relay.md

## 11. 완료 기준
- 계획서 병합본 존재, 과제 1개 확정, 임계값 확정 (실행 전)
- 조건 A1/A2/B/C 출력 및 토큰 수치 기록 (thinking·가시 출력 분리)
- 중간 저장 파일(원 입력, 압축 입력, caveman 원문, 번역본) 존재
- sonnet-reviewer 루브릭 판정 파일 존재, 라벨 매핑 공개 기록
- 결론 문서: 이득/손실 수치, 귀속(압축 vs caveman·번역), 위험 발현 여부, 스타일 준수 여부, "1회 실행이라 일반화 불가" 한계 명시
- 사용자에게 존댓말로 요약 보고 (파일 경로만 제시)

## 12. 미반영
(없음. 검토 16건 모두 반영. 단, 8절 체크리스트는 CLAUDE.md 4행만 근거이며, 단가 캐시 항목은 billing.md에서 확인되지 않아 5.3에 조건부 처리)
