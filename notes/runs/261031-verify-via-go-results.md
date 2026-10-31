# 261031-verify-via-go-results
plan: notes/runs/261031-plan-verify-via-go.md (review: -review.md)
tools: /tmp/ccscan (Go stdlib scanner, 1178 jsonl / 75632 lines, bad 0), /tmp/ccexp (E1-E3), /tmp/ccscan_s9.txt
data: ~/.claude/projects/-Users-leekwangseok-PycharmProjects-claude-harness (top cli 4 files / sdk-cli 162, sub cli 43 / sdk-cli 969)
summary: 확인 5 (2,3,4,5,10) / 부분 6 (1,6,7,8,11,13) / 불가 2 (9,12)

item1 -p 계상: 부분. metrics usage 쌍 1개뿐, 1% 증가당 턴 토큰 중앙값 6472.8 (표본 1, 사용자 동시 사용 오염 가능). 같은 기준 여부는 미확인
item2 시스템프롬프트·도구 동일성: 확인 — 다름. 메인 스레드 cli(repl_main_thread) system 12867-13160자, 도구 15-16개, 도구 정의 69104-74654자 / sdk-cli(sdk) system 8327-11170자, 도구 13개, 도구 정의 25906-26094자. cli 추가 도구: Artifact, SendFeedback, SendMessage. 첫 턴 프롬프트 토큰 opus: cli 44652 (n=1) vs sdk-cli 중앙값 29693 (n=14). 비교 시 통제 필요
item3 /usage 단위: 확인 — 계정 단위. /tmp와 저장소에서 동시 호출 값 동일 (session 8%, week 60%). /usage 자체 토큰 0, cost 0. 세션별 토큰은 트랜스크립트 sessionId로 귀속 가능, %는 귀속 불가
item4 인터랙티브 usage 기록: 확인 — cli assistant 472/472 (top), 1179/1179 (sub) 줄에 message.usage 있음. 기록 시점 세부는 미확인
item5 캐시 TTL: 확인(요청 기준) — 메인 스레드는 두 모드 모두 ephemeral_1h만 사용 (cli 765636, sdk-cli 9810139 토큰), 서브에이전트는 두 모드 모두 ephemeral_5m만 사용. 실제 만료 시점 실험은 미실시
item6 유휴 토큰 소모: 부분 — 트랜스크립트에서 5분 넘는 간격 뒤 사용자 입력 없이 생긴 요청 0건(S7). 트랜스크립트에 남지 않는 호출은 미확인
item7 프로세스 부하: 부분 — -p 자식 1개 RSS 152.5MB, CPU 8.0%. interactive로 분류된 2개는 10.1/20.9MB인데 실제 node 본체인지 미확인. 8개 동시 부하는 미측정
item8 동시 한도·429: 부분 — 과거 rate_limit/429/overloaded 흔적 sdk-cli 서브 assistant 2, user 1, blob 2. 부하 시험은 사용량 소모로 보류
item9 PTY 훅·설정 병합: 불가 — Go에 PTY 실행 없음 (creack/pty 없음). 옵션 B 필요
item10 -p stream-json /clear: 확인 — 처리됨. 이벤트 conversation_reset(trigger clear) 후 새 session_id, /clear 턴 토큰 0. 다음 턴 cache_read 11504 / cache_creation 10834 -> prefix 캐시 재사용됨
item11 유휴 task-notification: 부분 — 두 모드 모두 기록 존재 (top cli user 63, sdk-cli user 139, queue-operation 585). 유휴 중 전달 타이밍은 미확인
item12 tui 출력 격리: 불가 — 데이터 검증 항목이 아닌 설계 항목
item13 인터랙티브 부가 호출: 부분 — querySource 값은 repl_main_thread / sdk / agent:custom:* 만 존재, 제목 생성·compact 등 부가 호출 0건(S9d). 트랜스크립트가 부가 호출을 기록하는지 자체는 미확인

implications:
- 모드 비교의 최대 교란 변수는 item2: 인터랙티브 메인 스레드 프롬프트가 약 15k 토큰 큼 (도구 정의 차이). ABA 비교 시 첫 턴 cache_creation 차이를 별도 항목으로 보고하거나 도구 집합을 맞춰야 함
- /usage %는 계정 합계라 8개 상주 세션 동시 운영 중에는 모드별 분리 불가 -> 토큰은 트랜스크립트 sessionId 합산으로 비교
- PTY(옵션 B)가 필요한 건 item9와 item7·13의 완전 확인뿐
