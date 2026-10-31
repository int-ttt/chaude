# caveman 기반 새 구조 설계 계획서

status: 설계안 v7 (구현 코드·측정 절차 미포함)
date: 2026-10-31
history: v7 = 사용자 답변 반영(U5 캐시 비공유, U8 = c 원문+요약 확정, U3·U4·U7 보류) / v6 = 사용자 답변 반영(U1·U2·U6·U9·U10 결정, U3·U4·U5·U7·U8 재설명 대기) / v5 = 사용자 결정 반영(Q1 범위 한정, Q2 역할 정의, Q3 병합 시점), 14절 사용자 답변 필요 항목 추가 / v4 = summary_ko 조건 문구 통일, 상태 머리 위치 정정, 정합성 확인 항목 추가 / v1-v3 = 이전 커밋 이력 참조
scope: 하네스 내부 메시지 경로를 "caveman 채널(형식 고정 축약)"과 "사용자 채널(한국어 존댓말)"로 분리하는 구조 설계
not_scope: 구현, 측정·튜닝·비용 재측정, 이전 조건 A/B/C 비교 재실행

## 0. 이전 결과에서 가져온 제약 (근거만 인용, 재측정 없음)

- C1: 자유 축약형 전체 caveman은 비채택. 한정어·검색 범위·잘림 여부가 먼저 사라짐 (notes/runs/261030-plan-caveman-relay-read-review.md:8, :14-15)
- C2: 형식 고정 반환 스키마만 조건부 검토 대상 (notes/runs/261030-plan-caveman-relay-read.md:40-47, notes/runs/261030-plan-caveman-relay-flow.md:52)
- C3: 한정어(only, except, not, unless, 미확인)는 압축 금지. 예: "36번만 해당" -> "36 해당"은 의미 변경 (read.md:20)
- C4: 번역 전용 hop 추가 금지. scout가 같은 실행 안에서 한국어 요약을 직접 씀 (read.md:25). 중계 단계 3개 이상이면 절감 상쇄 가능성 높음 (flow.md:83)
- C5: opus-architect는 Agent 도구만 있어 [ws:<id>] 줄을 직접 낼 수 없음. 라우팅 주체는 메인 세션/하네스 (notes/runs/261030-plan-caveman-relay-flow-review.md:23)
- C6: 서브에이전트는 부모 맥락을 기본 상속하지 않음(fork만 상속), Agent 호출 model 인자는 alias만 (notes/runs/261030-plan-caveman-relay-structure-review.md:5). 따라서 필요한 맥락은 브리프에 직접 적어야 함
- C7: CLAUDE.md:4 "Korean, 존댓말."은 서브에이전트에도 적재됨. 에이전트 파일만 바꾸면 규칙 충돌 (notes/runs/261030-plan-caveman-relay-impl-review.md:8)
- C8: 차단형 훅은 실행 중 세션·코딩 작업을 막을 수 있음. allowlist 밖 전부 차단 금지, 구현은 별도 worktree에서만, stash 금지 (impl-review.md:5, :10, :15, :38)
- C9: 비용 근거는 조건 A 1회뿐 (total_cost_usd=0.5437, output=6196; notes/deprecated/261030-run-caveman-relay.md:5). B·C 미실행, 모델 단가는 billing.md에 없음 (같은 파일 :9). 따라서 본 구조의 절감량은 미확인
- C10: 현 구조는 이미 "메인 -> opus-architect(Agent만) -> haiku-*" 패턴 (structure-review.md:22). 새 구조는 이 골격을 유지하고 메시지 형식만 바꿈
- C11: 현 에이전트 파일 11개에는 이미 "Agent-read files (notes/*, runs/): terse English `key: value`, Korean values verbatim; user-read text: Korean." 규칙이 있고, 최종 반환 메시지는 "Korean 존댓말"로 정해져 있음 (.claude/agents/*.md 약 14-25행, 행 번호는 파일마다 다름). CLAUDE.md:4는 범위 한정 없이 "Korean, 존댓말.". 즉 파일 출력은 이미 축약형이고, 새 구조가 바꾸는 대상은 에이전트 간 브리프와 반환 메시지임
- C12: 4000자 반환 상한 때문에 scout가 문맥을 생략하고 Doubts에만 적은 관측 있음 (read-review.md:16). 잘림은 필드로 드러나야 함
- C13: opus 서브에이전트는 haiku-* 에이전트만 호출 가능 (PreToolUse:Agent 훅 오류 "sonnet/opus 서브에이전트는 haiku-* 에이전트만 호출할 수 있습니다 (v0.8.2)", 2026-10-31 본 계획 작성 중 관측). 따라서 L1 에이전트끼리 직접 호출 없음 [확실]

## 1. 목표

- G1: 사람이 읽지 않는 hop(에이전트 간 브리프·반환)을 형식 고정 축약으로 통일
- G2: 축약해도 근거(경로:행), 한정어, 검색 범위, 잘림 여부는 필드로 강제 보존
- G3: 사용자에게 가는 텍스트만 한국어 존댓말. 번역 전용 단계를 새로 만들지 않음 (C4)
- G4: 기존 에이전트 체계·모델 고정·메인 스레드 가드 동작은 그대로 둠
- 절감 목표 수치: 정하지 않음. 구현 후 측정해 판단 (U1 = a, 결정됨; C9)
- 원칙: 서브에이전트는 자기 결과만 부모에 전달하며, 부모와 캐시를 공유하지 않음 (U5, 결정됨)

## 2. 구조 개요: 채널 3층

| 층 | 경로 | 언어·형식 |
|---|---|---|
| L0 사용자 채널 | 입력: 사용자 -> haiku-prompter -> opus 세션 / 출력: haiku-tui -> 사용자 | haiku-tui 출력만 한국어 존댓말 자연문. haiku-prompter는 사용자 프롬프트를 opus 세션에 전달(사용자 원문 verbatim, 존댓말 규칙 비적용, 전달문 = 원문 + 요약, U8 = c 결정됨) |
| L1 지휘 채널 | 메인 <-> opus-architect | caveman 브리프(B형식) / 결정 반환(R-decide) |
| L2 작업 채널 | opus <-> haiku-* | caveman 브리프(B형식) / 읽기·쓰기 반환(R-read, R-write) |

- rule_6 (Q2 결정): haiku-prompter = 사용자 프롬프트를 opus 세션에 전달하는 역할. haiku-prompter 전달문 형식 = (c) 사용자 원문 + 요약: 원문 verbatim 그대로 + 요약 줄을 함께 전달 (U8 = c, 결정됨). haiku-tui = 사용자에게 답변하는 에이전트. 한국어 존댓말 표기는 haiku-tui 출력에만 적용. 아래 표·절의 "메인 세션" 중 사용자 대면 출력은 haiku-tui가 맡는 것으로 해석함 (해석 확인됨: 261031 사용자 "u8 confirmed correct")
- rule_7 (U10 = b, 결정됨): 현행 haiku-prompter의 반복·재귀 루프 소유 역할은 유지하고 사용자 프롬프트 전달 역할을 추가함

- rule_1: caveman은 L1·L2 전용. L0으로 나가는 문장은 그 문장을 쓰는 마지막 작성자가 처음부터 한국어로 씀
- rule_2: 반환 하나에 "caveman 본문 + 한국어 요약 1~5줄(summary_ko)"을 함께 둠 (필수 조건은 rule_4). 메인은 summary_ko만 사용자에게 전달하고 본문은 번역하지 않음. summary_ko는 현 에이전트 파일의 "최종 메시지 = Korean 존댓말 요약" 규칙 자리를 그대로 씀 (C11). 바뀌는 것은 최종 메시지가 "한국어 요약만"에서 "caveman 본문 + summary_ko"로 바뀌는 것뿐
- rule_3: 중계 단계 수는 현 구조보다 늘리지 않음 (C4)
- rule_4: summary_ko는 메인 직속 반환에만 씀. 브리프가 lang: ko이면 summary_ko 필수, lang: caveman이면 생략 (L2 반환은 상위 에이전트가 읽으므로 한국어 불필요). 메인은 항상 lang: ko, opus는 haiku 브리프에 lang: caveman (U7 결정됨(구현 조사, tool_input.prompt로 lang: ko 확인))
- rule_5: 중첩 hop에서 상위 에이전트는 하위 반환을 자기 반환에 E<n> 근거 행으로 다시 씀. 하위 반환 전문을 그대로 붙여 올리지 않음

## 3. 메시지 형식 정의

### 3.1 브리프 B형식 (하향)

```
task: <한 줄 동사구>
paths: <절대경로>[:<행범위>], ...
ctx: E<n>: <경로>:<행> / <수치 또는 인용 요지>   (상속 없음, C6)
limits: <수정 금지 경로>, <금지 행위>
return: R-read | R-write | R-decide, <상한(자 또는 줄)>
lang: caveman | ko   (ko = summary_ko 필수, rule_4)
```

- ctx의 E<n> 근거 행 형식은 이전 정의를 재사용 (notes/deprecated/261030-plan-caveman-relay.md:16-19)
- 사용자 원문·한국어 값은 ctx 안에서도 verbatim

### 3.2 읽기 반환 R-read

```
claim: <경로>:<행> | <사실 또는 원문 인용> [확실|추정|불확실]
searched: <본 경로·패턴>
not_found: <찾았으나 없음 확인>
not_checked: <안 본 범위>
truncated: yes|no ; omitted: <생략 항목 또는 ->
summary_ko: <lang: ko일 때 필수, 1~5줄>
```

- not_found와 not_checked는 합치지 않음 ("없음"과 "안 봄" 구분, read-review.md:15)
- 반환 상한에 걸리면 truncated: yes와 omitted 목록을 반드시 씀 (C12)

### 3.3 쓰기 반환 R-write

```
changed: <경로 목록>
diffstat: <git diff --stat 요지>
check: <명령> -> PASS|FAIL
ws: <작업 공간 id | none>
summary_ko: <lang: ko일 때 필수, 1~5줄>
```

- 필드 목록은 flow.md:58-61 정의를 그대로 씀

### 3.4 결정 반환 R-decide

```
decision: <한 줄>
why: <근거 E<n> 참조>
rejected: <버린 안: 이유>
risks: <항목>
open_q: <사용자만 답할 수 있는 질문 | ->
plan_path: <경로 | ->
summary_ko: <lang: ko일 때 필수, 1~5줄, L0 전달용>
```

### 3.5 caveman 허용·금지 범위

- 허용: 조사·접속어·인사·머리말 생략, key: value, 명사구, 영어 키
- 금지: 한정어 삭제(C3), 수치의 단위·대상 생략, 경로·행 생략, 확실도 표시 삭제, not_found/not_checked 병합, 한국어 값 번역·축약
- 빈 필드는 키를 지우지 않고 "-"로 둠 (필드 누락과 "해당 없음" 구분)

### 3.6 공통 상태 필드 (3.2~3.4 각 블록 맨 앞에 붙는 공통 머리 2줄)

```
status: done | partial | fail | blocked
reason: <partial·fail·blocked일 때 한 줄, 아니면 ->
```

- blocked: 훅·권한 거부로 도구 사용이 막힌 경우. 거부 메시지를 verbatim 인용하고 우회 없이 반환
- fail: check FAIL 또는 작업 불가. 다시 시도한 횟수를 reason에 적음
- 메인은 status가 done이 아니면 사용자 답변에 그 상태를 한국어로 반드시 적음

## 4. 에이전트별 적용표

| 에이전트 | 받는 형식 | 내는 형식 | 비고 |
|---|---|---|---|
| 메인 세션 | L0 한국어 | L1 B형식 | 사용자에게는 summary_ko만 전달 (사용자 대면 출력은 haiku-tui 정의 따름) |
| opus-architect | B형식 | R-decide | 계획 파일 본문은 노트 규칙(영어 key: value) 따름 |
| haiku-scout, haiku-researcher | B형식 | R-read | 1단계 적용 대상 |
| haiku-coder, haiku-batch-worker, haiku-runner | B형식 | R-write | |
| haiku-checker | B형식 | R-write (check 필드 중심) | |
| haiku-drafter | B형식 | R-write | 산출 문서 자체는 한국어 유지 (L0 산출물) |
| haiku-extractor | 기존 | 기존 | 파일 출력 전용, 변경 없음 |
| haiku-prompter | 사용자 원문 | 원문 verbatim + 요약 (U8 = c) | 존댓말 검사 제외. 사용자 프롬프트를 opus 세션에 전달, 한국어 존댓말 비적용 (Q2 결정). 현행 반복 루프 역할 유지 + 전달 역할 추가 (U10 = b, 결정됨). B형식 브리프 필수 5키(task, paths, limits, return, lang), ctx 선택 (A2) |
| haiku-tui | B형식 또는 R-* 반환 | L0 한국어 존댓말 자연문 | 사용자에게 답변하는 에이전트. 한국어 존댓말은 이 출력에만 적용 (Q2 결정). 에이전트 파일은 구현 단계에 신설 (U9 = a, 결정됨) |

## 5. 규칙 충돌 해소 (CLAUDE.md:4, C7)

- O1: CLAUDE.md 4행 범위를 "사용자에게 보이는 텍스트"로 한정하고 에이전트 간 반환은 caveman 형식이라고 명시 (현 에이전트 파일의 agent-read/user-read 구분과 같은 방식, C11)
  - 장점: 단일 출처, 모든 에이전트 일관
  - 단점: 전역 규칙 변경, 사용자 승인 필요
- O2: haiku-scout.md, haiku-researcher.md의 최종 메시지 규칙("Korean 존댓말" 줄)을 "R-read 본문(caveman) + summary_ko(Korean 존댓말)"로 바꾸는 예외 1줄 추가, 나머지 불변 (read.md:69)
  - 장점: 변경 범위 최소, 되돌리기 쉬움
  - 단점: CLAUDE.md와 충돌 상태 유지, L2 읽기 경로에만 적용
- O3: 브리프 lang 필드로 호출마다 지정, 파일 변경 없음
  - 장점: 파일 변경 0
  - 단점: 매 브리프 비용, 누락 시 한국어로 회귀
- choice (Q1 결정, 사용자): O1 승인. CLAUDE.md:4 범위를 "사용자에게 보이는 텍스트(haiku-tui 출력 포함 L0)"로 한정, 에이전트 간 반환은 caveman 형식. 전역 CLAUDE.md 파일은 이번에 수정하지 않고 계획에만 반영. 실제 수정은 S5. S5 전 과도기에는 O2(+ 브리프 lang 필드 병기) 유지
- why: 사용자가 Q1에서 범위 한정(O1)을 승인함. O2는 과도기 한정 예외이며 S5에서 제거

## 6. 형식 감시 (차단 아님)

- component: 신규 훅 .claude/hooks/caveman-format.py (이름 제안)
- 위치: Agent 호출 반환 직후 검사 (PostToolUse 대상 도구 matcher 이름은 Agent|Task (U3 결정됨))
- mode: 환경변수 CAVEMAN_GUARD=log|warn|off, 기본 log. impl.md:80 원안은 log|enforce|off였으나 enforce는 차단이므로 C8에 따라 제외하고 비차단 warn으로 대체. 원안 log는 "차단했을 호출" 기록, 여기서 log는 "필수 필드 누락" 기록으로 의미가 다름
  - log: 누락 필드를 notes/metrics/<날짜>.jsonl에 1행 기록
  - warn: 호출자 맥락에 "필수 필드 누락: <키>, 재요청" 1줄 주입
  - 어느 모드에서도 도구 실행 차단·종료 코드 차단 없음 (C8)
- 검사 항목: 형식별 필수 키 존재, truncated 값 yes|no, claim 행의 "<경로>:<행> |" 패턴, status 값 4종 중 하나, lang: ko 브리프의 반환에 summary_ko 존재 (tool_input.prompt로 확인, U7 결정됨)
- 검사 불가 항목: 한정어 손실은 정적 검사로 판별 불가 -> 감시 범위 밖으로 명시
- 감시 대상 언어 규칙 (Q2 결정): 한국어 존댓말 검사는 haiku-tui 출력에만 적용. haiku-prompter 전달문(전달문 = 원문 + 요약, U8 = c)은 B형식 필수 키(task·paths·limits·return·lang) 검사 대상이고 존댓말 검사 제외. 검사 불가 항목은 그대로

## 7. 라우팅·작업 공간 경계

- [ws:<id>] 표기와 세션 재개는 메인/하네스 책임 (C5). parseWS(internal/harness/orchws.go)는 입력 줄의 [ws:<id>] 태그용이며 routeOrch는 현재 호출되지 않음(b4855b7). R-write의 ws 필드(ws: <id> | none)는 하네스가 파싱하지 않음
- 상위 에이전트는 R-write의 ws 값을 다음 브리프에 "ws: <id> 이어서"로 적기만 함
- mode f는 기본 실행에서 live 자식을 둠 (repl.go:1090, :204-208; --clients-file=""일 때만 꺼짐) -> ws 필드는 none 허용

## 8. 적용 단계 (설계 순서, 실행 아님)

- S0 정본: 형식 정의 정본 1개(3.1~3.6)를 docs/claude-lean/CAVEMAN_FORMAT.md에 둠. 각 에이전트 파일에는 자기 반환 템플릿 블록만 복사 (상속 없음 C6, 매번 파일 읽기 비용 회피)
- S1 읽기 경로: haiku-scout, haiku-researcher에 R-read 템플릿 + O2 예외줄. 같은 단계에서 감시 훅(6절)을 log 모드로 등록하고 검사 대상은 S1 에이전트만
- S2 쓰기 경로: haiku-coder, haiku-batch-worker, haiku-runner, haiku-checker, haiku-drafter에 R-write 템플릿
- S3 지휘 경로: opus-architect에 B형식 작성 규칙과 R-decide 템플릿
- S4 감시 확대: 감시 훅 검사 대상 목록에 S2·S3 에이전트 추가 (목록은 훅 설정 1곳에서 관리)
- S5 통합 (Q1 승인 반영): CLAUDE.md:4 범위를 사용자 노출 텍스트로 한정(O1), O2 예외줄 제거. 전역 CLAUDE.md는 이번 계획 갱신에서 수정하지 않음
- 공통: 단계마다 별도 worktree 브랜치·별도 커밋, 되돌리기는 해당 단계 커밋 revert. master 작업 트리에서 직접 적용 금지 (C8)
- 병합 (Q3 결정): caveman 구현(S0~S5)이 끝난 뒤에 master로 병합. 병합 전까지 단계별 worktree 브랜치 유지. 단계별 부분 병합은 하지 않는 것으로 해석함 (해석, 확인은 U6과 함께)
- 시작 절차 (U6 = b, 결정됨): worktree는 커밋된 HEAD 기준으로 분기. master의 미커밋 변경은 worktree에 포함하지 않음
- haiku-tui 에이전트 파일(.claude/agents/haiku-tui.md) 신설은 구현 단계에서 수행 (U9 = a, 결정됨)

## 9. 바꾸지 않는 것

- main-thread-guard.py, agent-model-pin.py 동작
- fork 미사용, subagent_type은 정의된 에이전트만 (structure.md:30-31)
- L0 한국어 존댓말 규칙
- haiku-drafter가 만드는 사용자 문서의 언어
- notes/* 기존 기록과 이전 계획 파일

## 10. 설계 정합성 확인 항목 (정적 확인, 측정 아님)

- 각 에이전트 파일의 반환 템플릿이 정본과 문자열 일치
- CLAUDE.md·에이전트 파일 간 언어 규칙 충돌 0건 (S5 전에는 O2 예외 2파일만 허용)
- 훅 기본값 log, 모든 모드에서 차단 경로 없음
- haiku-drafter 산출물·L0 경로는 한국어 유지
- 모든 반환 템플릿(3.2~3.4) 맨 앞에 3.6의 status·reason 2줄 존재

## 11. 위험

- R1: 짧은 작업에서 필수 키 때문에 반환이 오히려 길어질 수 있음. 완화: 빈 값 "-" 허용, summary_ko 선택
- R2: 영어 키와 한국어 값 혼합으로 Haiku가 값까지 영어로 바꿀 수 있음. 완화: 3.5 금지 항목에 명시
- R3: 정본과 에이전트 파일 사본 불일치. 완화: 10절 문자열 일치 확인
- R4: summary_ko와 본문 내용 불일치(요약이 한정어 누락). 완화: summary_ko에도 C3 적용
- R5: 감시 훅이 형식만 보고 내용 누락은 못 잡음 (6절 검사 불가 항목)
- R6: 메인(Haiku)이 B형식 브리프를 잘못 쓰면 하위 전체가 형식을 벗어남. 완화: task·paths·limits·return·lang 5키만 필수, ctx는 맥락이 필요할 때만

## 12. 사용자만 답할 수 있는 질문

- Q1 [결정됨]: O1 승인. CLAUDE.md:4 범위를 사용자 노출 텍스트로 한정 (5절, 8절 S5 반영)
- Q2 [결정됨]: haiku-prompter = 사용자 프롬프트를 opus 세션에 전달. haiku-tui = 사용자에게 답변. 한국어 존댓말은 haiku-tui 출력에만 (2·4·6절 반영)
- Q3 [결정됨]: caveman 구현이 끝나면 병합 (8절 반영). 원래 Q3(worktree 기준점에 master 미커밋 변경 포함 여부, impl.md:161 U10)은 답이 없어 14절 U6으로 이관

## 13. 미확인

- 본 구조의 토큰·비용 절감량 (U1 = a: 목표 수치 없이 구현 후 측정해 판단)
- 모델별 단가 (billing.md 미기재) (U2 = b: 공식 문서 조사를 의뢰하는 것으로 기록, 조사 자체는 본 계획에서 하지 않음)
- 반환 검사용 PostToolUse matcher 이름 (U3 결정됨: Agent|Task)
- mode f 기본 활성 여부 (U4 결정됨: 기본 활성)
- 서브에이전트 캐시 공유: 결정: 공유하지 않음 (U5, 결정됨)
- haiku-tui 에이전트 파일이 현재 존재하지 않음 (현 에이전트 목록에 없음) -> 구현 단계에 신설 (U9 = a, 결정됨)
- 현행 haiku-prompter 정의는 반복·재귀 요청의 루프 소유자로, Q2 새 정의(프롬프트 전달)와 다름 -> 반복 루프 역할 유지 + 전달 역할 추가 (U10 = b, 결정됨)

## 14. 사용자 답변 필요 항목

(Q1~Q3 외 미확인·결정 항목 묶음. 각 항목 질문 1개 + 선택지. 본문 13장 등은 그대로 둠)
- 결정됨: U1, U2, U5, U6, U8, U9, U10 (U5 = 캐시 비공유, 서브에이전트는 자기 결과만 부모에 전달; U8 = (c) 원문 + 요약)
- 결정됨(구현 조사): U3, U4, U7 (261031)
- 미확인: U8 해석 확인 1건 ("사용자 대면 출력은 haiku-tui가 맡는다" 해석은 사용자 답변에 없어 맞음으로 간주하지 않음)

- U1 (13장 1, 1절): 절감 목표 수치를 정하시겠습니까? [결정됨: (a)]
  - 선택지: (a) 정하지 않고 구현 후 측정해 판단 (b) 목표 수치를 직접 지정 (c) 조건 B/C 측정을 먼저 하고 결정
- U2 (13장 2): 모델별 단가 자료를 제공하시겠습니까? [결정됨: (b)]
  - 선택지: (a) 단가를 직접 제공 (b) 공식 문서 조사를 의뢰 (c) 단가 없이 토큰 수 기준으로만 판단
- U3 (13장 3, 6절): 반환 검사 훅의 PostToolUse matcher 이름을 어떻게 확인하시겠습니까? [결정됨(구현 조사): PostToolUse matcher = Agent|Task, settings.json PreToolUse:50과 동일]
    - 맥락: 반환 검사 훅이 걸릴 도구 호출 이름(PostToolUse matcher)을 아직 확인하지 못함
  - 선택지: (a) 구현 단계에서 조사 (b) 지금 조사 의뢰 (c) 훅 없이 진행하고 6절 보류
- U4 (13장 4, 7절): mode f(--clients-file)를 기본으로 사용하십니까? [결정됨(구현 조사): 기본 실행에서 활성, repl.go:1090 기본값 ~/.chaude/clients.json, --clients-file=""일 때만 꺼짐; ws 키 필수, 값 none 허용]
    - 맥락: 기본 실행에서 mode f의 live 자식이 켜지는지 확인하지 못함(--clients-file 설정 시에만 켜짐)
  - 선택지: (a) 사용 (b) 미사용 (c) 모름, 구현 전에 확인
- U5 (13장 5): 서브에이전트 캐시가 부모와 공유되는지 조사할까요? [결정됨: 공유하지 않음; 서브에이전트는 자기 결과만 부모에 전달]
    - 맥락: 결정으로 캐시 공유를 전제하지 않음 (절감 계산에서 캐시 공유 전제 없음)
  - 선택지: (a) 구현 전 조사 (b) 조사 없이 진행 (c) 구현 후 비용 측정으로 판단
- U6 (구 Q3, 8절): worktree 기준점을 무엇으로 하시겠습니까? [결정됨: (b)]
  - 선택지: (a) master 미커밋 변경 포함 (b) 커밋된 HEAD 기준 (c) 미커밋 변경을 먼저 커밋한 뒤 분기
- U7 (6절): lang: ko 브리프 여부를 훅이 볼 수 있는지 불확실한데, summary_ko 필수 검사를 어떻게 하시겠습니까? [결정됨(구현 조사): PostToolUse 입력에 tool_input.prompt 포함, lang: ko 줄 있으면 summary_ko 검사]
    - 맥락: 훅이 lang: ko 브리프 본문을 볼 수 있는지 불확실하여 summary_ko 필수 검사 가능 여부를 모름
  - 선택지: (a) 검사 보류 (b) 구현 단계에서 확인 후 결정 (c) summary_ko를 항상 필수로 변경
- U8 (4절, Q2 후속): haiku-prompter가 opus 세션에 보내는 전달문 형식은 무엇으로 하시겠습니까? 그리고 "메인 세션의 사용자 대면 출력을 haiku-tui가 맡는다"는 2절 해석이 맞습니까? [결정됨: (c) 원문 + 요약. 해석 확인됨: 맞음 (261031)]
    - 맥락: 전달문 형식 외에, 2절의 "사용자 대면 출력은 haiku-tui가 맡는다" 해석이 맞는지 확인 필요
  - 선택지: (a) 사용자 원문 verbatim + B형식 머리 (b) 사용자 원문만 (c) 원문 + 요약 포함. 해석 확인: 맞음 / 아님, 정정
- U9 (13장 신규): haiku-tui 에이전트 파일은 어떻게 하시겠습니까? [결정됨: (a)]
  - 선택지: (a) 구현 단계에 신설 (b) 기존 에이전트로 대체 (c) 별도 계획서로 분리
- U10 (13장 신규): 현행 haiku-prompter(반복 루프 소유자)는 어떻게 하시겠습니까? [결정됨: (b)]
  - 선택지: (a) 새 정의로 대체 (b) 루프 역할 유지 + 전달 역할 추가 (c) 별도 에이전트로 분리

## 구현 점검 (실제 소스 기준)

| 항목 | 상태 | 근거(file:line) | 비고 |
|---|---|---|---|
| I1 | DONE | docs/claude-lean/CAVEMAN_FORMAT.md:5,19,33,45,57,63 | 3.2~3.4 블록 안에 3.6 머리 2줄 없음 |
| I2 | PARTIAL | opus-architect.md:28-33 | haiku-prompter 블록 6키 없음; §4 표 예외 여부 확인 필요 |
| I3 | DONE | haiku-scout.md:25-32, haiku-researcher.md:22-29 | - |
| I4 | DONE | haiku-coder.md:24-30, haiku-batch-worker.md:23-29, haiku-runner.md:24-30, haiku-checker.md:22-28, haiku-drafter.md:26-32 | - |
| I5 | DONE | opus-architect.md:37-45 | - |
| I6 | MISSING | haiku-prompter.md | 머리 2줄·R-블록 없음 (I2와 같은 원인) |
| I7 | DONE | haiku-tui.md:1-16, haiku-prompter.md:27-36 | - |
| I8 | DONE | CLAUDE.md:4 | - |
| I9 | PARTIAL | haiku-scout.md:16, haiku-researcher.md:15 | 일반 존댓말 문구가 R-read 절과 충돌 잔존 |
| I10 | DONE | .claude/hooks/caveman-format.py:105-131 | log/warn/off만, exit 2 경로 없음 |
| I11 | DONE | caveman-format.py:13-64, :89 | 주의: claim 검사는 R-read만 |
| I12 | DONE | .claude/settings.json:80-88 | PostToolUse Agent|Task |
| I13 | MISSING | internal/harness/orchws.go:11,14-20 | parseWS에 none 분기 없음; [ws:none]은 id "none"으로 통과 |
| I14 | PARTIAL | main-thread-guard.py:32, agent-model-pin.py | 마지막 변경 b4855b7; §9 "동작 안 바꿈"과 충돌, 사용자 확인 필요 |
| I15 | DONE | haiku-scout.md:25-32 == CAVEMAN_FORMAT.md:22-27, :66-67 | diff 0 |
| I16 | DONE | tests/hooks | python3.10 -m pytest 33 passed; 기본 python3(3.9)에는 pytest 없음 |
| I17 | DONE | master: ef90cf9, 2f2685b, d4e4b32, 401e5a1, bcc9f8e, ae2cbf0, 39db4ad, 0ee3501, 27c60e2 | 잔존 worktree 3개, 브랜치 6개 미삭제 |

요약: 점검 결과 DONE 12, PARTIAL 3, MISSING 2입니다 (항목 수 17).
MISSING은 I6(haiku-prompter 머리·R-블록 없음)과 I13(orchws none 분기 없음), PARTIAL은 I2, I9, I14입니다.
I6과 I2는 같은 원인(haiku-prompter 블록 6키 및 머리 2줄 누락)에서 비롯되어 한 가지 수정으로 함께 해결됩니다.
