# caveman 선택지 C 설계·오케스트레이션 계획서

status: 계획안 (코드 변경 없음, 실행 전 사용자 확인 필요 항목 §7)
date: 2026-10-31
input: plan/caveman-structure.md (v7, 255행 전체 읽음; 2절 채널 3층, 14절 U1~U10)
source_scope: notes/runs/261031-scope-router-sonnet-delete-channel3.md (선택지 A/B/C 정의, C 선택)
archive_rule: 삭제 대상은 원본을 notes/archive/261031/ 아래에 먼저 보관한 뒤 작업 트리에서 제거 (원본 삭제 금지)

## 1. 사용자 선택과 범위

- 선택: 선택지 C
  - 라우터 삭제: internal/harness/classifier.go, internal/harness/haikupool.go, internal/harness/routerprompt_test.go, internal/harness/classifier_model_test.go
  - sonnet 모델 관련 요소 삭제 (에이전트 파일, 가격 행, 라우팅 문구, 훅 대상, 테스트 픽스처 포함)
  - L1 지휘 채널 = opus-architect 단독 + haiku-* (sonnet-analyst·sonnet-reviewer 제거)
  - 계획서 2절 채널 3층을 아래 §3처럼 반영
- 범위 밖 (명시): 릴리즈 삭제·재릴리즈, 버전 번호 변경, 태그, CHANGELOG 릴리즈 항목 작성. 이번 계획은 코드·문서 변경 계획까지만 다룬다.
- 범위 밖 (기존 계획 유지): caveman 형식 자체(3.1~3.6 정의), 측정·비용 재측정(C9, U1=a).

## 2. 채널 3층 재정의 (선택지 C 반영)

| 층 | 현행 (2절 v7) | 선택지 C 이후 |
|---|---|---|
| L0 사용자 채널 | 입력: 사용자 -> haiku-prompter -> opus 세션 / 출력: haiku-tui -> 사용자 | 라우터 없음. 입력은 메인 Haiku 세션이 직접 받음. haiku-prompter 전달 역할 유지(U8=c, U10=b). haiku-tui 출력 유지 |
| L1 지휘 채널 | 메인 <-> opus-architect / sonnet-analyst / sonnet-reviewer | 메인 <-> opus-architect 단독. opus-architect가 여러 haiku-* 하위 에이전트를 병렬 호출해 결과를 받음 |
| L2 작업 채널 | opus·sonnet <-> haiku-* | opus-architect <-> haiku-* (haiku-scout, haiku-researcher, haiku-batch-worker, haiku-coder, haiku-checker, haiku-runner, haiku-drafter). sonnet 호출자 없음 |

- 호출 가능 관계 (기존 훅 제약 유지, C13): sonnet/opus 서브에이전트는 haiku-*만 호출. opus-architect -> haiku-* 는 허용 (agent-model-pin.py:27 SUB_CALLERS에 이미 opus-architect 포함, :40 허용 목록은 haiku-* 만)
- 중계 단계 수: 현행 "메인 -> sonnet-analyst -> haiku-*"와 "메인 -> sonnet-reviewer" 모두 깊이 2 이내. 선택지 C도 "메인 -> opus-architect -> haiku-*" 깊이 2. 증가 없음 (rule_3 충족, §4).
- rule_5 유지: opus-architect는 하위 haiku 반환을 E<n> 근거 행으로 다시 써서 메인에 R-decide로 반환. 하위 반환 전문 전달 금지.

## 3. 삭제 대상 파일 목록

| # | 경로 | 사유 | 줄 수 (현재) |
|---|---|---|---|
| D1 | internal/harness/classifier.go | 라우터 본체 (routeResult, classify, pickModel, route) | 299 |
| D2 | internal/harness/haikupool.go | 라우터 후보 풀 (workerPool, noteEdit) | 88 |
| D3 | internal/harness/routerprompt_test.go | 라우터 모델 env 테스트 | 75 |
| D4 | internal/harness/classifier_model_test.go | 라우터 프롬프트 테스트 | 39 |
| D5 | internal/harness/conductor_test.go | 전부 parseRoute·classify·workerPool·hint 테스트 (라우터 전용, D1·D2 삭제 시 컴파일 불가) | 확인 필요 (구현 단계 wc) |
| D6 | .claude/agents/sonnet-analyst.md | sonnet 에이전트 (L1 제거) | 확인 필요 |
| D7 | .claude/agents/sonnet-reviewer.md | sonnet 에이전트 (L1 제거) | 확인 필요 |

- D5, D6, D7은 4개 목록 밖의 추가 제안. 이유: D1·D2 삭제 시 D5가 컴파일 실패하고, "sonnet 관련 요소 모두 삭제"에 D6·D7이 직접 해당. 사용자 확인 대상(§7 Q-A).
- 삭제 전 필수: D1~D7 각각을 notes/archive/261031/ 하위에 복사 보관 후 제거.

## 4. 채널 3층별 변경 파일 목록

### 4.1 L0 사용자 채널

| 파일 | 행 | 변경 |
|---|---|---|
| internal/harness/repl.go | 174 | pool 필드 삭제 (D2 타입 제거) |
| internal/harness/repl.go | 576 | r.pool.noteEdit 호출 삭제 |
| internal/harness/repl.go | 878-879 | Conductor 분기에서 r.route(line) 호출 삭제 |
| internal/harness/repl.go | 1083 | -conductor 플래그 삭제 (help 문구 포함) |
| internal/harness/repl.go | 1138, 1143 | newWorkerPool 호출 삭제, status JSON의 "conductor" 키 삭제 |
| internal/harness/repl.go | 1204-1207 | setConductor 함수 삭제 |
| internal/harness/repl.go | 1213-1218 | /mode conductor, conductor-off 처리 삭제, 표시문에서 conductor 항목 삭제 |
| internal/harness/repl.go | 61 | Repl 설정 필드 Conductor 삭제 |
| internal/harness/tui.go | 171 | /mode 도움말의 "conductor|conductor-off = Sonnet conductor" 삭제 |
| .claude/agents/haiku-prompter.md | 12 | LOOP_TARGETS: sonnet-analyst, sonnet-reviewer 제거, opus-architect만 남김 |
| CLAUDE.md | 33 | "Chaude conductor" 단락 삭제 또는 opus-architect 단독 흐름으로 재작성 (전역 규칙 변경: 사용자 승인 필요) |

### 4.2 L1 지휘 채널

| 파일 | 행 | 변경 |
|---|---|---|
| .claude/agents/opus-architect.md | 3, 12, 15 | description·본문의 sonnet-analyst ESCALATE 전제 제거. "여러 haiku 하위 에이전트 병렬 호출 후 병합" 절차로 교체 |
| .claude/agents/sonnet-analyst.md | 전체 | 삭제 (D6) |
| .claude/agents/sonnet-reviewer.md | 전체 | 삭제 (D7) |
| CLAUDE.md | 8, 9, 10, 11, 13, 16 | 라우팅 문구의 sonnet-* 를 opus-architect로 교체. "Opus delegating to Opus saves nothing" 문구(9) 재검토 (사용자 승인) |
| .claude/hooks/agent-model-pin.py | 8-11 | docstring의 sonnet-reviewer 예외 문구 정리 |
| .claude/hooks/agent-model-pin.py | 27 | SUB_CALLERS에서 sonnet-analyst, sonnet-reviewer 제거 (opus-architect 유지) |
| .claude/hooks/agent-model-pin.py | 40, 43 | 허용 목록에서 sonnet-* 제거, 메시지 문구 수정 |
| .claude/hooks/agent-model-pin.py | 49 | sonnet/opus 서브에이전트 문구 중 sonnet 제거 (opus 부분 유지) |
| .claude/hooks/agent-model-pin.py | 57-59 | E gate: "sonnet-/opus- 는 ESCALATE 없으면 차단". 선택지 C에서 opus-architect가 기본 L1이면 이 게이트와 충돌. 판단 필요 (§7 Q-B) |
| .claude/hooks/agent-model-pin.py | 83, 87, 92 | 안내 문구·kind 판별에서 sonnet 제거 |
| .claude/hooks/main-thread-guard.py | 32 | 차단 안내 문구의 reasoning->sonnet-analyst, final review->sonnet-reviewer 를 opus-architect로 교체 |
| .claude/hooks/caveman-format.py | 11 | TARGETS에서 sonnet-analyst, sonnet-reviewer 제거 |
| internal/harness/orch.go | 38, 40, 44 | orchRuleB/C/E의 sonnet-analyst 문구를 opus-architect 기준으로 교체. rule E의 "no Sonnet/Opus agents except escalation" 문구 재작성 (§7 Q-B 결과 반영) |
| plan/caveman-structure.md | 39, 128-129, 135, 175, 210·211 (Q) | L1 행, 4절 sonnet 행, S3 단계의 sonnet-* 표기를 opus-architect 기준으로 수정. 14절에 신규 결정 항목 추가 (§7) |

### 4.3 L2 작업 채널 (haiku-* 및 모델 공통 요소)

| 파일 | 행 | 변경 |
|---|---|---|
| .claude/hooks/haiku-context-guard.py | 4, 6, 29, 55 | "sonnet/opus/비-haiku" 문구는 opus 기준으로 유지. CLAUDE_HARNESS_ALLOW_SONNET_IO 환경변수 이름 변경 여부 판단 (기본: 이름 유지, 값 의미만 정리) |
| .claude/hooks/ledger.py | 41, 94 | 모델 정규식은 과거 기록 파싱용. 삭제 시 과거 metrics 해석 깨짐. 기본 제안: 유지 (§7 Q-C) |
| internal/harness/budget.go | 89, 94 | sonnet 가격 행 삭제 여부. 과거 usage 기록 집계와 충돌. 기본 제안: 유지 (§7 Q-C) |
| internal/scripts/usage_report.go | 15, 18 | priceFams·가격 행 동일 쟁점. §7 Q-C와 함께 결정 |
| internal/harness/repl.go | 27-28, 863-864 | opusSonnetThreshold: sonnet 메인 회전 임계값. sonnet 메인 제거 시 opus만 남김 |
| internal/harness/multiproc_s_test.go | 273-404, 470 | "sonnet" 모델 키 픽스처를 opus 또는 haiku로 교체. classifyFn 스텁 테스트(371-378)는 D1 삭제와 함께 제거 |
| internal/harness/dispatch_test.go | 16-48 | "sonnet" 픽스처 -> "opus" 치환 (동작 불변 확인) |
| internal/harness/children_test.go | 15-22 | 동일 |
| internal/harness/clients_test.go | 19, 71, 76 | 동일 |
| tests/hooks/test_orch_gate.py | 확인 필요 | sonnet 또는 classif 문구 매치. 구현 단계에서 행 확인 |
| docs/claude-lean/*.md (FRESH_START, APPLY_GUIDE, README, CHANGELOG) | 확인 필요 | sonnet 언급 목록화 후 사용자 확인 (기록성 문서는 원칙적으로 변경 안 함) |

## 5. 보존 원칙 (변경하지 않는 것)

- main-thread-guard.py, agent-model-pin.py의 haiku-* 차단·pin 동작 (문구만 수정)
- haiku-tui, haiku-prompter 신설 상태와 역할 (plan 4절)
- caveman R-read / R-write / R-decide 형식 (plan 3절)
- notes/ 기존 기록, notes/deprecated, notes/archive 기존 항목
- haiku-drafter 산출 문서의 한국어 규칙

## 6. 비용·품질 trade-off: opus-architect가 sonnet-analyst·sonnet-reviewer를 대체하는 안

### 6.1 비용

- 단가 근거: internal/harness/budget.go:92-94 (opus in 4 / out 20, sonnet in 2 / out 10, 단위 $/MTok, alias 가정, 공식 단가 미검증. plan C9)
- 같은 토큰량 기준 opus는 sonnet 대비 약 2배. 따라서 L1 판단 토큰이 그대로 opus로 옮겨가면 비용 증가
- 상쇄 요인: opus가 직접 읽지 않고 haiku 하위 에이전트에 읽기·요약을 위임하면 opus 입력 토큰이 줄어듦. 그러나 병합 단계의 opus 입력(하위 반환 전부)은 남음
- 결론: 절감·증가 방향 모두 미확인. 실측 없이 판단 금지 (U1=a, 구현 후 측정)

### 6.2 품질

- 장점: 설계·판단 품질은 opus가 높음. 리뷰 단계 판단 주체 일원화
- 위험 1: sonnet-reviewer의 독립 검토 역할이 사라짐. opus-architect가 haiku로 증거를 모은 뒤 스스로 판정하므로, 구현 주체(haiku)와 판정 주체(opus)는 분리되지만 "재검토자"가 별도 에이전트가 아님
- 위험 2: haiku 결과 병합 시 근거 행(E<n>) 누락 가능 (R4, rule_5). 누락 검사는 caveman-format.py 감시(plan 6절, 차단 아님)에만 의존
- 위험 3: CLAUDE.md:9-10의 "opus는 ESCALATE 경로만" 원칙과 정면 충돌. 이 원칙을 바꾸면 전역 규칙 변경이므로 사용자 승인 필요

### 6.3 중계 단계 수 (rule_3)

| 경로 | 현행 | 선택지 C |
|---|---|---|
| 분석 | 메인 -> sonnet-analyst -> haiku-* -> sonnet-analyst -> 메인 (깊이 2) | 메인 -> opus-architect -> haiku-* -> opus-architect -> 메인 (깊이 2) |
| 리뷰 | 메인 -> sonnet-reviewer -> haiku-* -> 메인 (깊이 2) | 메인 -> opus-architect -> haiku-* -> 메인 (깊이 2) |
| 라우팅 | 메인 -> classifier(일회성 sonnet 호출) -> 메인 | 제거 (라우터 호출 0회, 단계 1 감소) |

- rule_3 판정: 충족. 깊이 증가 없음, 라우터 제거로 단계 1 감소

### 6.4 rule_3 충족 여부 요약

- 충족. 단, 라우터의 일회성 classifier 호출이 사라지므로 전체 hop 수는 줄어듦. 이는 절감 근거가 아니라 구조 단순화 근거로만 기록

## 7. 사용자 확인이 필요한 항목 (실행 전 필수)

- Q-A: D5(conductor_test.go 전체), D6·D7(sonnet 에이전트 파일)을 4개 대상 목록에 추가 삭제해도 되는가
  - 선택지: (a) 추가 삭제 승인 (b) D5만 승인, 에이전트는 유지 (c) 보류
- Q-B: agent-model-pin.py:57-59 E gate (sonnet-/opus- 는 ESCALATE 없으면 차단)를 어떻게 하는가. opus-architect가 기본 L1이 되면 모든 호출에 ESCALATE 문구가 필요해짐
  - 선택지: (a) 게이트 유지, 브리프에 ESCALATE 포함 강제 (b) opus-architect 호출은 게이트 예외 (c) 게이트 제거 (rule E 재작성 포함)
- Q-C: 과거 기록 해석용 sonnet 항목 (ledger.py:41·94, budget.go:94, usage_report.go:15·18, 가격 행)을 유지하는가 삭제하는가
  - 선택지: (a) 유지 (과거 metrics 해석 보존, 기본 제안) (b) 삭제 (과거 sonnet 기록은 미집계로 처리)
- Q-D: "sonnet 모델 관련 요소 모두 삭제"에 Go 모델 키 "sonnet" 자체(repl.go 메인 모델 전환, /model sonnet, childKey 픽스처)를 포함하는가
  - 선택지: (a) 포함: 모델 전환 경로에서 sonnet 완전 제거 (영향 큼, 테스트 픽스처 치환 필요) (b) 라우팅·에이전트·문구만 제거, 모델 전환 경로는 유지
- Q-E: CLAUDE.md 전역 규칙 수정 (4.1 행 33, 4.2 행 8-16) 승인
  - 선택지: (a) 승인 (b) 계획만 반영, 파일은 보류
- Q-F: §6의 비용·품질 trade-off를 보고 opus-architect 단독 L1 채택을 확정하는가
  - 선택지: (a) 확정 (b) 측정 후 결정 (U1=a 방식 준용) (c) 취소, 선택지 A 또는 B로 되돌림

## 8. 실행 단계 (Haiku 에이전트별 위임)

원칙: 단계마다 별도 worktree 브랜치(커밋된 HEAD 기준, U6=b), 단계별 커밋, 되돌리기는 해당 단계 revert. master 작업 트리 직접 수정 금지 (plan 8절, C8). stash 금지. 차단형 훅 추가 금지.

| 단계 | 위임 에이전트 | 위임 항목 | 반환 형식 |
|---|---|---|---|
| P0 기준선 | haiku-scout | 현 HEAD 기준 go test ./... 결과 행 수·실패 목록, .claude/hooks/tests 결과, sonnet 참조 행 전수 목록 (§4 표 검증용) | R-read |
| P0 기준선 | haiku-checker | 같은 HEAD에서 go test ./... 와 hooks 테스트 실행, PASS/FAIL 기록 | R-write (check) |
| P1 보관 | haiku-batch-worker | D1~D7 원본을 notes/archive/261031/ 아래 복사 (삭제 금지, 복사만) | R-write |
| P2 라우터 제거 | haiku-coder | D1·D2·D3·D4·D5 삭제; repl.go 174·576·878-879·1083·1138·1143·1204-1218·61 및 tui.go:171 수정 (§4.1) | R-write |
| P2 검증 | haiku-checker | go build ./... 와 go test ./... 실행, 컴파일 오류 행 보고 | R-write (check) |
| P3 sonnet 에이전트 제거 | haiku-batch-worker | D6·D7 삭제; haiku-prompter.md:12, opus-architect.md:3·12·15 수정 | R-write |
| P3 훅 문구 | haiku-coder | agent-model-pin.py (27·40·43·49·83·87·92), main-thread-guard.py:32, caveman-format.py:11, haiku-context-guard.py 문구 수정. E gate(57-59)는 Q-B 결과에 따라 수정 | R-write |
| P3 검증 | haiku-checker | .claude/hooks/tests 실행 (test_caveman_format.py 포함), tests/hooks/test_orch_gate.py 실행 | R-write (check) |
| P4 Go 모델 치환 | haiku-coder | Q-D 결과에 따라 테스트 픽스처 sonnet -> opus 또는 haiku 치환 (multiproc_s_test.go, dispatch_test.go, children_test.go, clients_test.go); orch.go 38·40·44 문구 수정 | R-write |
| P4 검증 | haiku-checker | go test ./... 전체 실행 | R-write (check) |
| P5 전역 규칙 | haiku-batch-worker | Q-E 승인 시 CLAUDE.md 행 8-16, 33 수정. 미승인 시 변경 없이 목록만 반환 | R-write |
| P6 계획 문서 | haiku-drafter | plan/caveman-structure.md 2절·4절·5절·8절 S3, 13·14절 sonnet 표기를 선택지 C 기준으로 수정. 14절에 Q-A~Q-F 항목 추가 | R-write (산출 문서 한국어 유지) |
| P6 정합 확인 | haiku-scout | 전체 저장소에서 sonnet, classifier, haikupool, routerprompt, conductor 재grep (exclude .git, logs), 남은 행을 허용·불허로 분류 | R-read |
| P7 최종 검증 | haiku-checker | go test ./..., .claude/hooks/tests, tests/hooks/test_orch_gate.py 전체 실행 후 결과 표 | R-write (check) |

- opus-architect는 실행 단계에 직접 투입하지 않음 (설계는 이 계획서로 결정됨). 재설계가 필요해지면 opus-architect 호출 1회만 허용 (ESCALATE 사유 기록)
- sonnet 호출 없음 (선택지 C 결과)

## 9. 검증 명령

| 시점 | 명령 | 통과 기준 |
|---|---|---|
| 모든 Go 변경 단계 | cd /Users/leekwangseok/PycharmProjects/claude-harness && go test ./... | 실패 0, 컴파일 오류 0 |
| 훅 변경 단계 | .claude/hooks/tests 실행 (테스트 러너 형식은 구현 단계 첫 행에서 확인 후 확정) | 실패 0 |
| 오케스트레이션 게이트 | tests/hooks/test_orch_gate.py 실행 | 실패 0 |
| 정합 확인 | grep -rn -i "sonnet\|classifier\|haikupool\|routerprompt" --exclude-dir=.git --exclude-dir=logs . | 허용 목록(§4.3 보관 항목) 외 0건 |

- 검증 실패 시: 해당 단계 커밋 전 중단, haiku-coder 재시도 1회, 그래도 실패하면 status: fail과 reason 기록 후 사용자에게 보고 (plan 3.6)

## 10. 위험과 완화

- R-A: 라우터 제거로 Repl 필드·설정 참조가 넓게 퍼져 컴파일 실패 다발. 완화: P2에서 go build 선행, 실패 행 목록화 후 재수정
- R-B: E gate 변경이 orch rule E와 어긋남. 완화: Q-B 결정 후 orch.go와 훅 동시 수정
- R-C: 과거 metrics 해석이 깨짐. 완화: Q-C (a) 기본 유지
- R-D: 전역 CLAUDE.md 변경이 모든 세션 라우팅에 영향. 완화: Q-E 승인 전 파일 미수정
- R-E: opus 비용 증가. 완화: 구현 후 측정 (U1=a), 목표 수치 없음

## 11. 미확인

- opus·sonnet 실제 단가 (budget.go 가정값, plan C9)
- 선택지 C에서의 실제 비용·품질 차이 (측정 전 미확인)
- tests/hooks/test_orch_gate.py 내 sonnet·classif 매치 행 (구현 단계 확인)
- .claude/hooks/tests 테스트 러너 형식 (구현 단계 확인)
- docs/claude-lean 문서의 sonnet 언급 범위 (구현 단계 목록화)
