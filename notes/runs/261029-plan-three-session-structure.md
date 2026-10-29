# 계획: chaude 3-세션 구조 (사용자 정정 반영)

date: 261029
status: 문서만, 코드 수정 없음
sources: notes/DECISIONS.md, notes/runs/261029-plan-hybrid-design.md, 리뷰 2건 (-review.md Opus, -review-analysis.md Sonnet)
표기: "가정" = 사용자 말에 없어서 채운 부분 (확인 필요). DECISIONS:N = 줄 번호.
용어: Opus 설계 세션 / 실행 세션 / 설계 파일 (하나로 통일).

## 사용자 정정 6점 (사용자 순서, 1회만 인용)
주의: 사용자 원문 문자열이 이 작업에 전달되지 않아, 이전 계획 문서에 기록된 문장을 그대로 옮김. 원문과 대조 필요.
1. Opus 설계 기획 세션은 항상 존재한다.
2. sonnet/haiku 분류자가 이 작업을 opus, sonnet, haiku 중 어느 실행 세션이 수행할지 고른다.
3. sonnet/haiku 실행 세션에 대해, Opus가 1시간 안에 비슷한 작업이 다시 올지 판단하고, 그 판단으로 각 세션을 유지할지 종료할지 Opus가 결정한다.
4. 분류자에게 준 프롬프트와 각 모델의 답변을 Opus에게 전달한다.
5. 컨텍스트 여유는 opus/sonnet 270k, haiku 80k.
6. sonnet 등 실행 세션은 Opus 데이터를 직접 가져가지 않는다. Opus가 설계 파일을 저장하고, haiku-scout가 그 파일을 보고 호출된다.

## 세션 구성 (점1)
- Opus 설계 세션은 항상 존재. 실행 세션은 opus/sonnet/haiku 중 분류자가 고른 하나.
- 충돌: DECISIONS:38 "main Haiku 고정", hybrid-design D1/(c) "ctx<T이면 Opus main". 채택: 사용자 정정. "T 이상이면 Opus main 없음" 규칙 폐기.
- 유지: DECISIONS:25 조사(haiku-researcher, haiku-scout)는 서브에이전트. DECISIONS:40(1) "Opus는 설계 턴 전용" 취지는 설계 세션 역할로 유지.
- 사용자 답변은 실행 세션이 한국어 존댓말로 쓴다 (DECISIONS:30). Opus 설계 세션 출력(keep/end 줄, 설계 파일 알림)은 사용자에게 보이지 않는다.
- 세션 훅: Opus 설계 세션 = main 훅 세트, 실행 세션 = worker 훅 세트 (DECISIONS:28, 가정).
- 카테고리 회전 (DECISIONS:29): 실행 세션에는 적용, Opus 설계 세션은 카테고리를 가로지르므로 제외 (가정).

## 분류자 흐름 (점2, 점4)
- 분류자는 `model: haiku|sonnet|opus` 줄을 추가로 출력한다 (현재 classifier.go:15-23은 topic/size/kind/worker/task만).
- 라우팅 규칙:
  - 작은 조회/수정 (kind=lookup|edit, size=small) -> haiku
  - 검토/분석 (주어진 자료의 review, analysis, comparison) -> sonnet (DECISIONS:35)
  - 설계/판단 (design, architecture, strategy, trade-off, 정정, 반박, 'opus로', 반복 실패) -> opus (DECISIONS:30, :33)
  - model 줄 누락/무효 -> size/kind로 위 규칙 적용 (size 기본 large와 같은 안전쪽 패턴)
  - pickModel/pickTarget의 Haiku 고정은 해제. 강제 --model은 여전히 우선.
- opus 선택 시: 별도 Opus 실행 세션을 만들지 않고 Opus 설계 세션이 직접 처리 (Opus 컨텍스트 이중 과금 방지). 가정, 사용자 확인 필요 (Q3).
- 충돌: DECISIONS:32 분류 1회 호출 후 즉시 해당 세션으로 전송, Opus 전달 단계 없음. 채택: 사용자 정정. 분류 호출 자체(1회, 도구 없음, metrics stage=classify)는 유지.
- 점4 전달 방식: 분류자 입력/답변은 Opus 턴마다 보내지 않고 파일에 기록 (아래 "파일 소유자" 표). Opus는 (i) 설계 턴, (ii) keep/end 판단 시점에만 이 로그를 읽는다. 작은 haiku 경로는 Opus를 깨우지 않는다.
- "각 모델의 답변" 해석: 분류자 모델의 답변 (실행 세션 답변 아님) 으로 둔다 (가정, Q2).
- 변하지 않는 것: 맞장구(ㅇㅇ)와 슬래시 명령은 현재 세션으로 (DECISIONS:32).
- 분류자 기본 모델: sonnet (conductorModel, 라이브 검증 DECISIONS:33). haiku 분류자는 TestClassifierLive 통과 후 (Q1).

## 유지/종료 규칙 (점3)
- 대상 단위: 살아 있는 sonnet/haiku 실행 세션 각각 (세션별 판단). opus는 설계 세션 자체이므로 대상 아님.
- Opus 입력: chaude가 세션 목록을 붙인다. 항목 = label(cat), files(workerPool), last_used(RFC3339), context size.
- "비슷한 작업" = 같은 [cat: X] 라벨 또는 workerPool 파일 겹침 (기존 데이터 재사용, 새 유사도 모델 없음).
- Opus 출력: 기계가 읽는 줄만, 세션당 하나. `keep: <id>` 또는 `end: <id>`. 파싱 실패/무응답 = end.
- 유지 창: last_used + 3570초 (캐시 TTL 1시간 고정, 유휴 핸드오프 59:59, DECISIONS:23). 이후에도 비슷한 작업이 없으면 Opus에게 다시 묻지 않고 자동 종료. 자동 종료는 가정 (코드에 없음: 유휴 회전은 non-test 코드에 미구현).
- 충돌: DECISIONS:19 (15분) 는 :21 (4m50s) 과 :23 로 대체됨. 인용을 :23 으로 교체. 15분 강제 초기화와 겹침은 Q4.
- 종료 시 백그라운드 에이전트가 있으면 기존 draining (DECISIONS:18) 을 그대로 쓴다.
- 구현 토대: 새 레지스트리 없이 parked + workerPool (classifier.go:183-210, haikupool.go) 위에 얹는다.

## 컨텍스트 한도 (점5)
- 한도: opus/sonnet 270k, haiku 80k (repl.go:26-27, :870). DECISIONS:41, hybrid-design D1 과 일치. 한도 도달 시 답변 후 rotate() + handoff, 백그라운드 에이전트가 있으면 draining (DECISIONS:18).
- 충돌: DECISIONS:40(2) "Opus/Sonnet main은 임계값으로 전환하지 않음" vs :41 및 코드(repl.go:27, :695)의 270k 회전. 채택 :41 (사용자 정정, 코드와 일치). :40(2) 개정 제안 -> Q5.
- 150k (CLAUDE.md:27) 는 일반 claude 자동 compact, 100k (CLAUDE.md:19) 는 서브에이전트 compact로 세션 회전 수치가 아니다. DECISIONS:14 의 300k 는 :16 로 대체. 자식 compact 창 400k (repl.go:871) 는 270k 보다 커야 270k 회전이 먼저 작동한다.
- 구현 주의: effectiveThreshold (repl.go:695) 는 현재 r.a.Model 하나만 본다. 세션 2개 동시 존재 시 세션별 임계값과 lastCtx 필요.
- Opus 설계 세션 270k 도달 시 핸드오프는 짧게: 설계 파일 경로 목록 + 미결 항목 (상태가 디스크에 있음).

## 데이터 흐름 (점6)
- 설계 파일: `notes/runs/<YYMMDD>-plan-<topic>.md`, chaude가 경로 결정 (topic = 분류자 task). 필수 절: `## evidence` (path:line + 이미 읽은 사실), `## brief` (haiku <=3,000자, sonnet <=6,000자, DECISIONS:41 개요 상한과 동일).
- 실행 세션은 설계 파일이 완료(존재 + size>0 + 2초 안정, DECISIONS:18)된 뒤 시작. 작은 작업은 대기 없음. brief는 chaude가 실행 세션 task에 그대로 붙인다.
- haiku-scout 호출 형태: 호출 주체 = Opus 설계 세션. 입력 = 설계 파일 절대경로 + 질문 1개. 출력 = 답변 5줄 이하, 읽기 전용, 원문 붙여넣기 금지. scout는 `## evidence` 에 없는 것만 조사. 설계 파일 쓰기는 Opus 쪽 (scout는 Read/Grep/Glob/Bash만).
- 일치: DECISIONS:39 (sonnet-analyst는 파일 직접 읽지 않음) 유지. Haiku 실행 세션은 설계 파일 1개를 직접 읽어도 됨 (가정).
- 충돌: hybrid-design D2 "Haiku가 개요 작성/관리". 채택: 사용자 정정. D2 는 보류하고 Opus가 설계 파일을 작성한다. 개요 파일 = 설계 파일 (같은 파일). Haiku 보조 역할 부활은 Q6.
- 사용자 프롬프트는 분류자 입력에 이미 있어 Opus에게 따로 재전송하지 않는다.
- 흐름: 1 프롬프트 -> 분류자 {model, task} / 2 로그 기록 / 3 (opus 또는 large) Opus가 설계 파일 저장 / 4 실행 세션이 brief 수령 / 5 결과는 notes/runs/ 에 쓰고 요약 반환 (DECISIONS:36).

## 파일 소유자
| 파일 | 쓰는 쪽 | 읽는 쪽 |
|---|---|---|
| 설계 파일 notes/runs/<YYMMDD>-plan-<topic>.md | Opus 설계 세션 | haiku-scout, chaude (brief 추출), 실행 세션 |
| 분류자 사본 (프롬프트+답변+세션 목록) route-log (상태 디렉터리 또는 notes/runs/<YYMMDD>-route-log.jsonl) | chaude (코드) | Opus 설계 세션 (설계 턴/keep-end 시점만) |
| 결과 파일 notes/runs/<YYMMDD>-<kind>-<topic>.md | 실행 세션 | 메인/사용자 |
| metrics notes/metrics/*.jsonl (stage=classify, keepend 추가) | chaude (코드) | 사람, 집계 스크립트 |

## 확인 항목 (구현 후)
- TestClassifierLive 에 sonnet 대상 프롬프트 3개 추가 (기존 6: opus 4, haiku 2). metrics classify 에 routed_model.
- 작은 haiku 프롬프트가 Opus 턴을 만들지 않는다 (Opus 턴 = 설계 턴 + 판단 시점).
- 큰 작업의 실행 세션은 설계 파일 mtime 이후에 시작.
- 영향 파일: classifier.go, repl.go, haikupool.go, tui.go (미커밋 변경 있음). hybrid-design D3 (백그라운드 패널 수정) 이 먼저. 순서: model 줄+매핑 -> route log+세션 목록 -> 설계 파일 대기 -> keep/end 파서(3570초) -> 세션별 임계값 -> 확인.

## 열린 질문
1. 분류자 기본 sonnet (제안) 이고 haiku 는 테스트 통과 후. 맞는가? (사용자 확인 필요)
2. Opus 전달 순서: 큰 작업은 설계 파일을 기다리는 안(제안). 분류 오판 시 Opus가 모델을 바꿀 수 있는가? "각 모델의 답변" 이 분류자 답변만인가?
3. model=opus 일 때 설계 세션이 직접 수행(제안)인가 별도 Opus 실행 세션인가?
4. 15분 무입력 강제 초기화(DECISIONS:19)는 :23 으로 대체됐다고 보고 폐기해도 되는가? 3570초 값 확인.
5. 컨텍스트 임계값 불일치: Opus 설계 세션이 항상 있으므로 hybrid-design (c) 의 T(150k/100k) 기반 Opus-main 전환은 폐기하고 270k/80k 로 통일 (제안). DECISIONS:40(2) 를 :41 에 맞춰 개정할 것인가? (DECISIONS:40 vs :41 충돌)
6. hybrid-design D2 의 Haiku 개요 관리를 설계 파일 보조로 남기는가?
7. Opus 설계 세션이 1시간 이상 쉬면 캐시 갱신 핑(59:59)으로 유지하는가, 만료 후 다음 설계 턴에서 다시 읽는가?
확인 우선순위: Q2, Q4 먼저, Q6 후순위.

## 반영 기록
- 반영: 사용자 6점 인용 블록 / model 줄+라우팅 / opus 선택 시 설계 세션 직접 처리 / keep-end 규칙+세션 목록+3570초+자동 종료 가정 / 컨텍스트 한도+DECISIONS:40 충돌 / 분류자 로그 파일화 / 파일 소유자 표 / haiku-scout 호출 형태 / Opus-hybrid 임계값 불일치 Q5 / 용어 통일 / D2 임시 기본값 / evidence-brief 절 / 충돌 정리(:24-30, :38 일부) / 확인 항목 / 질문 우선순위
- 보류(원문 없음): 사용자 6점 "원문 그대로" - 전달된 원문이 없어 이전 문서의 문장으로 대체, 메인이 원문 주면 교체
- 보류(범위 밖): :38 코드 영향(planHint, TestHaikuPlanWorkflow, CLAUDE.md:31) 상세 처리 - 구현 단계에서 결정
- 보류(분량): Q# 번호를 본문 가정 줄마다 달기, 점->절 대응표
