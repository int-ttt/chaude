# 상시 활성 세션 구성 검토 (261031)
제안: chaude당 기본 4 / 최대 8; 기본 = 메인 opus, tui haiku, coder haiku, 프롬프트 haiku; 남은 4 = coder 예약
판정: 이대로는 채택 보류. 기본 2 + 필요 시 생성 + 최대 6(상한을 실제 코드로 강제)으로 바꾸는 것을 권장
## 근거 (확인된 사실)
- 상한 없음: 261029-analysis-concurrency-limit.md:2 "DefaultMaxWorkers=3 is parsed but never used"; children.go에 자식 수 상한 상수 없음
- 자식 키 = childKey(cwd, model) (children.go:16) -> 같은 cwd의 haiku 역할 3~7개가 한 키로 충돌. liveChild.Role 필드는 있지만 키에 포함되지 않음
- 메인 규칙: CLAUDE.md:33 "main session is always Haiku ... Opus is for design only, never main"; DECISIONS.md:40 Opus 메인은 설계 턴에만
- 유휴: --idle-rotate 3599s, TTL 1h이면 3599s, 5m/미관측이면 290s (clients.go:148-160); ws 자식 wsMaxIdle 270s
- 단가 (FACTS.md:8,10, 입력/5m쓰기/1h쓰기/읽기/출력, $/MTok): Opus 4/5/8/0.20/20, Haiku 0.10/0.125/0.20/0.01/0.50
## (1) 4/8 개수의 타당성
- 프로세스가 살아 있어도 캐시가 따뜻하게 유지되지는 않음. 캐시 TTL은 요청할 때만 갱신되므로, 아무 요청 없이 띄워 둔 슬롯은 토큰을 아끼지 못함
- 같은 prefix(시스템 프롬프트·도구 정의)는 새 자식도 캐시를 그대로 읽음 -> 미리 띄운 예약 슬롯은 프로세스 기동 지연만 줄임 (토큰 이득 0)
- 실제로 드는 비용은 유휴 RSS(미측정)와 재기동 시 캐시 쓰기. 150k 컨텍스트를 1h 캐시로 다시 쓰는 비용: Opus $1.20, Haiku $0.03 -> 비용 위험은 사실상 Opus 슬롯에 몰려 있음
- 구독 5h 사용량 한도는 세션 전체가 함께 씀 -> 8개가 동시에 돌면 한도 소진이 빨라짐
## (2) 역할 분담과 실제 동시성
- tui haiku가 곧 chaude 메인(라우터)이므로, 메인 opus와 함께 두면 메인이 2개가 됨
- prompter는 반복 요청 때만 쓰는 루프 담당이고, coder는 "작업 흐름당 같은 인스턴스"를 이어 씀 -> 둘 다 필요할 때 생성하는 쪽이 맞음
- 실측 동시 자식 수 데이터 없음(분석 문서: 모델 약 3개 x cwd, 동일 haiku 자식 2개가 보인 이상 사례는 미검증) -> coder 예약 4자리는 근거 없음
## (3) 메인 충돌과 필요한 변경
- 상시 "메인 opus"는 CLAUDE.md:33·DECISIONS:40과 충돌함. Opus는 설계용 온디맨드 자식으로 두고 유휴 290s 후 회전시키는 것을 권장. 상시 메인으로 바꾸려면 사용자가 DECISIONS를 새로 정해야 함
- 필수 코드 변경: childKey에 role(또는 슬롯 번호) 포함; DefaultMaxWorkers를 실제 상한으로 연결하고 초과 요청은 대기열 처리; 역할별 유휴 기한(opus 290s, haiku 3599s)
## 권장 구성
- 상시 2: 메인 Haiku(TUI 겸 라우터) + coder Haiku 1
- 필요 시 생성: prompter Haiku, coder Haiku 추가분, Opus 설계 1(최대 1개)
- 최대 6 = 메인 1 + Haiku 작업자 4 + Opus 1. 8은 측정 후 RSS와 5h 한도가 허락할 때만
## 채택 전 측정 항목
- 유휴 자식당 RSS·CPU: ps -o rss,pcpu, 모델별 10분 샘플
- 회전 직후 첫 턴의 cache_creation_input_tokens(ephemeral_1h/5m 구분, 모델별): metrics jsonl
- 시간당 최대 동시 자식 수와 역할별 호출 빈도: notes/metrics/*.jsonl
- 콜드 기동 지연(spawn부터 첫 토큰까지), 미리 띄운 슬롯과 비교
- 병렬 자식 수에 따른 5h 사용률 증가 속도(usagepoll)
## 위험
- 키 충돌로 역할이 섞여 컨텍스트가 오염됨; 상한이 없어 프로세스가 폭주함; Opus 유휴 재기동 비용; 5h 한도 조기 소진
