title: 261031 measure results (passive, $0). spend: $0
source: notes/metrics 2026-10-28(84) 10-29(639) 10-30(383) = 1106 recs
M2 opus: turns 14, sessions 7, max 4 turns/session -> sessions with >=5 opus turns: 0 -> 계획 조건 충족 불가. overall opus: cache_creation 209801, cache_read 764975, output 20396, cost_usd 7.3978
M7 ctx delta (result_ctx, same session, zero-ctx excluded): haiku n=323 p50 5741 p95 192818 max 816792 (negatives 99, 원인 미확인; ctx-field delta p50 3316 p95 11068 max 52852); sonnet n=1 8148; opus n=7 p50 9031 p95 43913 max 43913
M7 rotate: n=41 (closed 33, drained 8). flags.threshold 80000 x22, 270000 x19. rotate_at_tokens == last turn ctx (literal overshoot 0 by construction). overshoot vs threshold (20 ctx-triggered rows): p50 4924 p95 13887 max 21631 min 1510; opus 271531 x4 => +1531. rotate_at_tokens values 37816..101631 for haiku, 80000 임계인데 37k~74k 에서도 회전 다수 (이유 미확인: 다른 reason 가능)
M7 post-rotation (heuristic match, n=41): first turn result_ctx p50 76888 p95 133432 max 199288; first-call cache_creation p50 28164 p95 45247 max 52875; cache_read p50 49172; ctx p50 39554; gap rotate end->first turn p50 73.9s p95 2514.8s
M6-지연 classify ms: all n=290 p50 4308 p95 8543 max 17160; haiku p50 4331, sonnet 3696, opus 4290. turn duration_ms: haiku n=297 p50 49877 p95 300004; sonnet n=8 p50 67337; opus n=14 p50 64062 p95 240172. rotate derived: p50 16288 p95 334009 max 599995. 홉 단위(전달 hop) 필드 없음 -> 홉별 지연 미확인
M5 상주 vs 새 생성: 미확인 (구분 필드 없음; turn.spawns 1건뿐)
M3 한영 토큰비: 미실행 (caveman 채택 전제, 2 이후)
M4 읽기 상한: 미실행
M1/계획 2 caveman B/C n>=3: 미실행 (유료, 조건 B/C 정의·사용자 예산 확인 필요)
M6 billing 비율: 미실행 (저우선)
doubts: 99 negative haiku deltas; post-rotation match heuristic; 10-28 coverage small; plan/review 번호 체계 불일치
채점 기준(근거·조건부 표현 누락): 아직 저장 안 함 (caveman 실행 전 작성 필요)

## 2차 측정 (80k haiku / 270k opus 기준, 기존 metrics 10-28~10-30, 신규 지출 $0)
data: turn 442, session 57, rotate 41, classify 290. hop/delegation 레코드 없음.
M2 opus: n=14 (10-29 n=13), cc=209,801 cr=764,975 in=70 out=20,396, 비용 $4.2522 (전체 $56.6134의 7.5%, 턴당 $0.3037). haiku n=420 $48.69 (86%), sonnet n=8 $3.67 (6.5%). 5회 이상은 10-29만 충족. 비용은 누적 cost_usd 차분으로 유도.
M5: resident vs fresh 직접 식별자 없음 -> 미확인. 대리지표: haiku 첫 턴 cc 중앙 27,492 / 이후 2,941; 비용/턴 중앙 0.0087 / 0.0392. opus 첫 cc 26,382 / 이후 3,322. background haiku 평균 $0.129 (지연 중앙 3.1s), foreground $0.128 (55s).
M6 지연: haiku p50 14.5s p95 300s(5분 상한 포화, 타임아웃성 46-47건; 제외 시 p50 10.9s p95 203s). sonnet p50 68.9s, opus p50 66.8s p95 189s. classify p50 4.3s p95 8.5s. 홉별 레코드 없음 -> 홉별 지연 미확인.
M7 ctx 증가/턴: haiku n=375 p50 2,989 p95 12,043 max 92,441; opus n=7 p50 3,322 p95 4,921; sonnet n=2 표본 부족. 회전 임계: 80,000 (16회, 10-29/30), 270,000 (4회 시도 모두 실패, 271,531, last_res=0). 오버슈트(ctx-임계, 80k): 중앙 7,818 / 최소 1,510 / 최대 21,631. rotate_at_tokens는 마지막 턴 ctx와 같아 문자 그대로의 오버슈트는 항상 0 (의미 없음). 회전 후 첫 호출 cc 중앙 29,778 (21,488-45,247).
100k 초과: haiku ctx 48/420 (11.4%), result_ctx 176/420 (41.9%); opus 0/14.
오류: classify 3/290 (1.0%, exit status 1). 0토큰 턴 약 59 (47건은 300s 타임아웃) - 스크립트 간 2건 불일치.
불확실: ctx와 result_ctx 의미 차이(ctx 차분이 일치하는 쌍 21/384), 8건 음수 증가분.
M3 (한국어 vs 영어 토큰 비율): 기존 데이터에 쌍 비교 없음 -> 미확인.
M4 (읽기 4000자 vs 무제한): 대조군 없음 -> 미확인.
M6-billing (5시간 사용률 vs API 단가 비율): 데이터 없음 -> 미확인.
M2-caveman B·C: 미실행. 사유: 조건 B/C 정의, 테스트 과제, 실행 명령이 어느 문서에도 없음 (참조 문서 261030-plan-caveman-relay.md, 261030-run-caveman-relay.md 파일 없음). 채점 기준 저장도 보류. 지출 $0.
총 지출: $0.
