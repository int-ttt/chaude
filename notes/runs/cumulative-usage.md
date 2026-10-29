# 누적 사용량 (2026-10-28 ~ 10-29)

source: ~/.claude/projects/-Users-leekwangseok-PycharmProjects-claude-harness/**/*.jsonl (243 files, 130 under subagents/)
period: 2026-10-28T10:10Z ~ 2026-10-29T01:45Z
unit: assistant 응답 1건(message.id+requestId 기준 중복 제거, 스트리밍 중복은 output 최대값 채택) = 1,818건

## 표 (토큰)
| 모델 | 구분 | input | output | cache read | cache write | 응답 수 |
|---|---|---|---|---|---|---|
| opus | main | 812 | 213,307 | 22,104,849 | 732,590 | 305 |
| opus | subagent | 278 | 91,304 | 4,861,141 | 389,510 | 132 |
| sonnet | main | 90 | 28,947 | 886,575 | 266,911 | 33 |
| sonnet | subagent | 1,956 | 716,076 | 22,273,609 | 2,528,146 | 927 |
| haiku | main | 448 | 129,679 | 7,359,870 | 1,181,054 | 204 |
| haiku | subagent | 442 | 112,397 | 1,992,579 | 497,867 | 215 |
| 합계 | | 4,026 | 1,291,710 | 59,478,623 | 5,596,078 | 1,818 |

total tokens (4종 합): 66,370,437
by model (4종 합): opus 28.40M / sonnet 25.70M / haiku 11.14M 근사 (아래 계산 참고)
- opus: 812+213,307+22,104,849+732,590 + 278+91,304+4,861,141+389,510 = 28,393,791
- sonnet: 90+28,947+886,575+266,911 + 1,956+716,076+22,273,609+2,528,146 = 26,702,310
- haiku: 448+129,679+7,359,870+1,181,054 + 442+112,397+1,992,579+497,867 = 11,274,336
- (합 = 66,370,437)
main vs sub: main 32.84M(opus 23.05M, sonnet 1.18M, haiku 8.67M) / subagent 33.53M

## 비용
cost: 기록 없음. 트랜스크립트에 costUSD 필드 0건, LEDGER.md에도 토큰/비용 필드 없음(텍스트 요약만, 196줄), metrics jsonl에도 없음. 단가를 임의로 가정하지 않아 달러 합계는 내지 않았습니다.

## 소스별 상태
- notes/metrics/2026-10-28.jsonl: turn 레코드 55건뿐, 토큰 합 = input 2 / output 4 / cache write 20,569 (haiku 1건 외 거의 0). 사용량 집계용으로 쓸 수 없음.
- notes/metrics/2026-10-29.jsonl: turn 26건, 토큰 필드 전부 0. 사용 불가.
- notes/LEDGER.md: 에이전트 결과 요약 텍스트. 토큰/비용 필드 없음.
- 따라서 유일한 실측 출처는 트랜스크립트. 세 소스는 겹치지 않음(metrics/LEDGER는 사실상 토큰 정보 없음).

## 불확실성 / 공백
- 트랜스크립트는 ~/.claude/projects 에 남아 있는 것만 집계. 삭제된 세션, 다른 프로젝트 디렉터리, 하니스가 별도 프로세스로 띄운 호출 중 이 디렉터리에 기록되지 않은 것은 빠짐.
- cache read가 90%(59.5M)를 차지. 요금제에서 cache read 가중치는 낮으므로 "총 토큰"은 실제 부담과 크게 다를 수 있음.
- 'main'은 파일 경로에 subagents/ 가 없는 것. haiku main 은 하니스가 메인 세션을 haiku로 돌린 턴으로 추정(추론).
- 모델 구분은 message.model 문자열 포함 여부(opus/sonnet/haiku). synthetic 2건은 토큰 0.
- 트랜스크립트가 압축/재개되며 같은 응답이 여러 파일에 중복될 수 있어 id로 제거했으나, id 없는 레코드는 중복 제거 불가(해당 사례는 확인 못 함).
