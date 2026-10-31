# 계획서: 상시 세션 구성과 interactive child 사용 타당성

status: 계획만 작성됨. 구현·측정 미실행. 버전: v2 (v1 대비: 비용 근거·측정 선행순서·가설 H1~H4 추가)
근거 파일(notes/runs/): 261031-plan-interactive-child(+review), 261031-plan-resident-sessions(+review), 261030-plan-live-sessions(+review), 261031-plan-live-session-composition-review
표기: [근거] 근거 파일에 있는 내용 / [추정] 해석 / 수치가 없으면 "미확인"

## 1. 목표
- G1. 구독 로그인 상태에서 "claude -p 호출이 일반 대화보다 사용량을 더 많이 소모한다"는 관찰이 사실인지 변인통제로 확인한다. [근거] interactive-child:4
- G2. 사실이라면 자식 세션을 interactive claude로 바꿀 가치가 있는지, 아니면 현행 stream-json 유지 + 보완(C)으로 충분한지 결정한다.
- G3. 상시 활성 세션 구성(사용자 제안: 기본 4 = main opus · tui haiku · coder haiku · prompter haiku, + coder 여유 4, 합계 최대 8)을 채택할지 결정한다.

## 2. 현재 상태 (근거)
- 자식 세션은 `claude -p --input-format stream-json`으로 실행됨 (core.go:145). [근거]
- 상한 없음: DefaultMaxWorkers=3은 파싱만 되고 쓰이지 않음 (261029-analysis-concurrency-limit.md:2), children.go에 자식 수 상한 상수 없음. [근거]
- childKey는 role 단일 문자열 키(children.go:16, childSet children.go:70)라 같은 role 5개가 충돌함. [근거]
- parked는 모델→세션 id 맵(repl.go:175, classifier.go:258-285)이라 haiku 역할 3개가 서로 덮어씀. [근거]
- /clear 경로는 endAllParked("clear")를 호출함 (repl.go:1168). 사용자 규칙은 /clear 금지. [근거]
- 유휴 회전: --idle-rotate 3599s, TTL 1h이면 3599s, 5m 또는 미관측이면 290s (clients.go:148-160); ws 자식 wsMaxIdle 270s. [근거]
- 실측 동시 자식 수 데이터 없음. [근거]
- 단가(FACTS.md:8,10, 입력/5m쓰기/1h쓰기/읽기/출력, $/MTok): Opus 4/5/8/0.20/20, Haiku 0.10/0.125/0.20/0.01/0.50. 150k 컨텍스트를 1h 캐시로 다시 쓰는 비용 Opus $1.20, Haiku $0.03 → 비용 위험은 Opus 슬롯에 집중. 유휴 RSS는 미측정. [근거] composition-review:9,13
- tui haiku가 곧 chaude 메인(라우터)이라 상시 main opus와 함께 두면 메인이 2개가 됨. [근거] composition-review:16
- 유휴 세션은 턴을 보내지 않으면 토큰을 쓰지 않는 것으로 보이나 미확인(13). 캐시는 TTL 경과 후 사라지므로 coder 여유 세션의 이점은 기동 지연 절감뿐. keepalive 핑은 비용이 생기므로 기본 끔. [근거/추정] resident-sessions:32

## 2-1. 사용량 증가 가설 (검증 대상, 모두 미확정)
- H1. 분류기·usagepoll 등 부가 호출이 -p 사용량으로 잡힘 (대응: 로컬 규칙 우선, usagepoll 주기 증가)
- H2. -p 자식 캐시 TTL이 5m이라 5분 넘게 쉬면 매 턴 재작성 (1h는 2배 단가)
- H3. 유휴 회전 시 캐시 갱신 비용 (대응: metrics에 별도 기록)
- H4. 자식 env에 ANTHROPIC_API_KEY가 상속되어 구독이 아닌 API 과금 (대응: env 제거)
출처: interactive-child-review.md. H1~H4는 모드 자체(서버 측) 차이 가설보다 먼저 배제한다.

## 3. 방법 후보
### 3-1. 자식 실행 방식
- A. PTY 화면 스크래핑: session_id·usage 취득 불가, 화면 변경에 취약. 제외 권고. [근거]
- B. PTY 입력 + 트랜스크립트 JSONL tail + Stop/PreToolUse 훅(--settings 주입): 스트리밍·cost 표시 일부 불가, 훅이 현재 main 세션에만 적용(DECISIONS.md:26), 코드 경로 2개. [근거]
- C. 현행 stream-json 유지 + 보완 (권장): ANTHROPIC_API_KEY를 자식 env에서 제거(-p는 키가 있으면 항상 그 키를 씀, billing.md:10), 자식에 1h TTL 강제(설정 키는 공식 prompt-caching 문서에서 확인), 분류기 호출 감소·usagepoll 주기 증가, 유휴 회전 캐시 갱신 비용을 metrics에 별도 기록. [근거]
- 결정 규칙: 측정(M3)에서 B로 갈 근거가 확인될 때만 B 시제품(한 모델 자식에만, 기본 끔, stream-json 경로 유지). [근거]

### 3-2. 상시 세션 구성
- S-fixed: 261030 계획. 기본 4 + coder 여유 4 = 8 슬롯. 여유 슬롯은 lazy spawn(미리 띄우지 않음), 유휴 270s 초과 시 종료. coder 묶음 한계 작업 5개 또는 ctx 24000. [근거]
- S-min: 261031 검토. 상시 2(메인 haiku=TUI 겸 라우터 + coder haiku 1), 필요 시 생성(prompter, coder 추가, Opus 설계 1), 최대 6(메인 1 + Haiku 작업자 4 + Opus 1). 상한은 코드로 강제. 8은 측정 후 RSS·5h 한도가 허락할 때만. [근거]
- S-ab(261031 resident-sessions): 운영 8개는 stream-json 상주 유지 + 별도 A/B 측정 쌍(interactive 1, -p 1). 8개 전부를 interactive로 바꾸면 계정 단위 집계에서 귀속 분리가 불가능해 변인통제가 오히려 어려움. [근거]
- 두 계획이 충돌: S-fixed(8, 상시 main opus)와 S-min(6, 상시 opus 아님). 상시 main opus는 CLAUDE.md:33·DECISIONS:40과 충돌하므로 채택하려면 사용자가 DECISIONS를 새로 정해야 함. [근거] 표기 해석은 미결(7절).

### 3-3. 어느 구성이든 필수 코드 변경 (구현은 이 계획 범위 밖)
- childKey에 role(또는 슬롯 번호) 포함
- DefaultMaxWorkers를 실제 상한에 연결, 초과 요청은 드롭하지 않고 대기열 처리 (dispatch.go:43은 현재 새 세션을 시작하지 않고 작업을 버림)
- parked 키를 모델이 아니라 역할 단위로 변경
- 역할별 유휴 기한(opus 290s, haiku 3599s)
- 실험 스크립트는 scripts/exp/ 독립 파일, internal/harness/* 수정 금지 [근거]

## 4. 미확인 항목
선행(측정 방향을 결정):
1. 5시간 사용률·/usage가 계정 단위 집계인지, 세션 귀속이 가능한지 (동시 세션 간 공유 여부, billing.md:12)
2. 구독 OAuth에서 -p 사용량이 구독 한도에 일반 대화와 같은 기준으로 계상되는지 (billing.md:18, 공식 문장 없음)
3. 실제 캐시 TTL(5m 또는 1h). Claude Code는 구독 한도 내 "main conversation"에만 1h TTL을 요청 (prompt-cache-ttl-clear.md:25); 1h write는 2배 단가(같은 파일:13-14)
4. 인터랙티브와 -p의 시스템 프롬프트·도구 정의 동일 여부(토큰 수·캐시 prefix)
5. 인터랙티브의 부가 호출(제목 생성, 자동 요약/compaction 등 백그라운드 모델 호출) 존재 여부
B 채택 시에만 필요:
6. 인터랙티브 트랜스크립트 JSONL의 경로·기록 시점·usage 필드
7. PTY 인터랙티브에서 --settings 훅 실행과 사용자 설정 병합
8. PreToolUse 훅이 can_use_tool을 대체 가능한지
9. 유휴 상태에서 task-notification 전달 여부
10. bracketed paste, 슬래시, ESC, 신뢰/온보딩 대화상자 처리
11. -p 없이 --output-format 무시 여부
12. 같은 세션을 -p 살아 있는 상태에서 --resume해도 안전한지
상시 구성용:
13. 유휴 상주 프로세스의 토큰 소모(백그라운드 요청 여부), 유휴 자식당 RSS·CPU, 8개 동시 부하
14. 계정 동시 요청 한도와 8개 동시 실행 시 429 빈도
15. -p stream-json에서 /clear가 슬래시 명령으로 처리되는지, 처리 후 prefix 캐시 재사용 여부
16. tui 출력 격리 구현 경로 (tui 출력이 opus main 컨텍스트에 들어가면 안 됨)
17. 구독 5h 한도는 모든 세션이 함께 씀 → 8개 동시 실행 시 소진 가속 정도 (수치 미확인)

## 5. 측정 설계
순서: M0 → M1 → M2 → C 보완 → M3. B는 M3 기준을 넘을 때만. (M0~M2 세부는 interactive-child-review.md 16~18행, 본 계획서는 미확인으로 둠)
선행 게이트: 미확인 1(계정 단위 집계 여부)을 가장 먼저 답한다. 이 답이 동시 실행 허용 여부와 측정 단위를 결정한다. 5h %는 정밀도가 낮아 프롬프트당 3회 반복하고 판정은 토큰 우선, %는 보조. [근거] resident-sessions-review:7-8
M3 (A/B, 소량 지출):
- 조건: 다른 세션이 없는 시간대, 같은 모델·cwd·고정 프롬프트 5턴 스크립트. (a) interactive claude vs (b) claude -p --input-format stream-json. 각 3회, 턴 간격 0초와 6분 두 조건. 분류기·usagepoll은 끈 상태. [근거]
- 실행 위치: 하네스 밖 별도 쉘, 메인 세션은 대기(메인 사용량이 측정 구간을 오염). 운영 풀 설정 변경은 측정 이후 별도 계획. [근거]
- 기록: 턴별 input, cache_creation(5m/1h), cache_read, output 토큰, 실행 전후 /usage 5시간 %. [근거]
- 주 지표: FACTS.md 단가 기반 토큰 가중합. 보조 지표: 5h % (정수라 거침). [근거]
- 캐시 통일: arm 사이 TTL 경과 대기(TTL 1h면 60분 이상) 또는 각 arm 첫 호출을 콜드로 간주해 분리 집계. [근거]
- 판정: 토큰 같고 %만 다르면 모드 자체 차이(서버 측). 토큰이 다르면 TTL·프롬프트 차이로 보고 C에서 고침. [근거]
- 지출 제한: 5시간 상한 60% 정책(DECISIONS.md:45) 준수, 측정만을 위해 상한까지 쓰지 않음. [근거]
상시 구성 측정: 모델별 10분 샘플 `ps -o rss,pcpu`로 유휴 RSS·CPU. [근거]

## 6. 수용 기준
- AC1. 미확인 1~5에 대해 각각 "확인됨/아님/미확인 유지"와 근거 출처가 기록된 결과 표가 있다.
- AC2. M3 결과 표(턴별 토큰, 가중합, 5h %)와 결정 규칙 적용 결과가 있다.
- AC3. B 시제품 기준: 동일 토큰 조건에서 5h % 차이 15% 이상, 3회 모두 같은 방향 [근거: interactive-child-review.md:24]. 이 외에는 C 유지. 별도 제안 임계값(-p 토큰 합계가 interactive 대비 10% 이상 많고 3회 방향 일치)이 있어 둘 중 하나를 사용자가 확정해야 함(D4). 확정 전에는 B 진행 금지.
- AC4. 상시 구성은 상한이 코드로 강제되고(초과 요청은 대기열), 키 충돌·parked 덮어쓰기가 해소된 뒤에만 채택.
- AC5. 사용자 확인 선행: B로 가면 실시간 스트리밍, 권한 대화(can_use_tool), cost 표시가 줄어드는 점을 사용자에게 먼저 확인받는다.

## 7. 미결 결정 (사용자 확인 필요)
- D1. 목적: "사용량 비교 실험"인가 "운영 자체를 interactive로 전환"인가.
- D2. "8개"는 상시 기동 수인가 상한인가. 상한이면 coder 여유 4는 기본 0개 사전 기동.
- D3. 상시 main opus 허용 여부(CLAUDE.md:33·DECISIONS:40 개정 필요) vs Opus 온디맨드(유휴 290s 후 회전).
- D4. 판정 임계값: 5h % 15%(+3회 일치) vs 토큰 10%.
- D5. 묶음 한계(5개/ctx 24000)는 시작값이며 notes/metrics로 조정(측정 전 근거 없음).
