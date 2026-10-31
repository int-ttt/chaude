# plan: 모델호출 정책 재구현 (라우터 완전 삭제)

status: draft (opus 설계 검토 대기)
대상 저장소: /Users/leekwangseok/PycharmProjects/claude-harness (branch master, 커밋 248b5e0 이후)
기준 문서: plan/caveman-structure.md

## 1. 사용자 결정 (확정)
- 라우터(classifier.go의 claude -p 분류기) 완전 삭제. 관련 파일: classifier.go(라우터 부분), routerprompt_test.go, classifier_model_test.go. 환경변수 CHAUDE_ROUTER_MODEL 및 [routing: ...] 힌트 상수 삭제.
- 기존 라우터 형식(분류 프롬프트, 태그 방식) 무시.
- 구조: haiku가 사용자 입력을 받아 opus에게 전달한다. sonnet 계층 없음.
- rotation 임계값: haiku 80,000 토큰, opus 200,000 토큰.
- CLAUDE_CODE_SUBAGENT_MODEL 및 롤백용 환경변수 관련 항목은 불필요. 다루지 않음.
- REPL 형태(입출력, 세션 전환 UI)와 관리 로직(세션 상태, 큐, 워커 풀 haikupool.go)은 유지.
- 재구현 범위: 모델 호출 정책 부분만. 대상은 repl.go의 모델 선택·rotation 임계값·route 경로, orch.go의 모드별 기본 모델, .claude/settings.json의 메인 모델 설정.
- 기준: plan/caveman-structure.md에 맞춰 처음부터 구현.
- 작업 위치: 원본 저장소. 복제본과 T 디렉토리는 건드리지 않음.

## 2. 현재 상태 (조사 결과, 원본 기준)
- classifier.go 15-16행: routerModelDefault="haiku", routerModelEnv="CHAUDE_ROUTER_MODEL".
- classifier.go 212-250행 route(): 분류 결과가 haiku면 switchSession("haiku"), 다르면 parked/rotation 경로.
- repl.go 27-28행: opusSonnetThreshold=150000. 862-866행 thresholdFor(model). 950-952행 rotate 조건. 1066-1069행 기본 -model=haiku, -threshold 150000, -compact-window 400000.
- repl.go 575행: haiku 세션 Edit 시 pool.noteEdit 호출(워커 풀, 유지).
- orch.go 23-30행 orchDefaultModel: b·c·d·f=opus, e=haiku, 나머지 빈 값.
- .claude/settings.json 2행 "model":"opus", 6행 CLAUDE_CODE_SUBAGENT_MODEL=haiku.

## 3. 설계 초안 (검토 대상)
- D1. 라우터 제거: classify()·route() 중 분류 호출 부분 삭제. 사용자 입력은 분류 없이 현재 세션 흐름으로 전달.
- D2. haiku 전달 구조: 메인 세션은 haiku. 사용자 입력을 opus 호출 경로로 넘기는 단일 지점을 둔다(repl.go route 대체). 판단 주체는 opus.
- D3. rotation: thresholdFor(haiku)=80000, thresholdFor(opus)=200000. sonnet 분기 제거. 기존 150000 상수 제거.
- D4. 기본 모델 통일: 메인 기본 haiku(플래그 기본값), settings.json model은 haiku로 맞출지 설계 검토 항목.
- D5. orch.go 모드 기본 모델은 유지하되 sonnet 참조가 있으면 제거.
- D6. 테스트: conductor_test.go, multiproc_s_test.go 등 라우터 힌트 문자열 검사를 제거하고, rotation 임계값 테스트를 80k/200k로 갱신.

## 4. 열린 질문 (opus 검토에서 답)
- Q1. D2의 "haiku가 opus에게 전달"을 코드로 어떻게 구현할지(세션 전환 vs 호출 래퍼).
- Q2. settings.json model을 opus로 둘지 haiku로 맞출지.
- Q3. haikupool.go를 유지할 때 라우터 전용 심볼(candidates, candidateText)을 어디까지 남길지.
- Q4. 롤백 경로(git revert만으로 충분한지).

## 5. 검증 계획 (실행 단계, Haiku 에이전트가 수행)
- go build ./... 와 go test ./internal/... 통과.
- pytest tests/hooks 통과.
- grep으로 CHAUDE_ROUTER_MODEL, classifyPromptSonnet, routing 힌트, sonnet 잔존 0건 확인.
- 임계값 테스트: haiku 79,999/80,000 경계, opus 199,999/200,000 경계.

## 6. 개선 반영 (병합)
형식: 번호 | 상태 | 반영 내용 (review 문서 기준, 본문 1~5절은 변경하지 않음)
- 1 | 반영 | route 제거. haiku→opus 전달은 Go 래퍼 없이 haiku 메인의 Agent 위임(opus-architect, frontmatter model opus)으로만 구현.
- 2 | 보고(D-A) | submit 신설 안 함. runTurn(repl.go:500-505)이 예산 게이트·spend 리셋·Send를 이미 수행하므로 text=line으로 runTurn 직행. 단일 전송점은 runTurn.
- 3 | 후속 | "haiku가 opus에 전달" 지시 위치(CLAUDE.md, .claude/agents, plan/caveman-structure.md) 확인은 미실행. 실행 결과 없으면 범위 밖 후속 항목.
- 4 | 반영(step3) | classifier.go 삭제: classify, route, parseRoute, parseRouteFull, routerModel(), routerModelDefault, routerModelEnv, classifyPromptSonnet/Haiku, [routing] 힌트 상수, classifyFn. switchSession만 유지.
- 5 | 완료(step1) | grep 결과: parked 유지(switch UI), switchSession 유지(orchws.go:62,74, switchchild.go:59,147), has 유지(conductor_test.go:201).
- 6 | 반영(step4) | haikuThreshold=80_000, opusThreshold=200_000. repl.go:1068 기본값 → haikuThreshold. 플래그는 haiku 덮어쓰기로 유지.
- 7 | 반영(step4) | thresholdFor: strings.Contains(model,"opus") → opusThreshold, 그 외 r.a.Threshold.
- 8 | 반영(step4) | repl.go:951 조건식 불변, 임계 출처만 교체. effectiveThreshold는 compact-window min을 취하지 않음(D-D, 사실 기록). 카테고리 rotation(942-946) 불변.
- 9 | 후속 | Claude Code 자동 압축 시점이 200k 이후인지는 실행 전 검증 불가. 실행 후 보고 항목.
- 10 | 반영(step6) | 저장된 model="sonnet" 세션 재개 시 80k 적용 테스트 1건 추가.
- 11 | 반영(step5) | haikupool.go: candidates, candidateText, maxCandidates 삭제. haikuWorker, workerPool, newWorkerPool, noteEdit, has 유지(has는 읽기 API).
- 12 | 반영(step7) | .claude/settings.json "model" opus→haiku. 6행 CLAUDE_CODE_SUBAGENT_MODEL 불변. internal/settings 테스트 실행.
- 13 | 확인 | orch.go orchDefaultModel에 sonnet 없음. 변경 없음.
- 14 | 반영(step2,6) | 삭제: routerprompt_test.go, classifier_model_test.go, multiproc_s_test.go TestRoute* 그룹. 재작성: TestMainRouteSwitchPolicy. 수정: TestWorkerPool. 신규: TestThresholdFor, rotate 경계 4건. 리터럴 grep 확인.
- 15 | 반영(step10) | sonnet 잔존 grep 범위: internal/harness/{repl,classifier,orch,haikupool}.go와 그 테스트. go vet ./... 추가.
- 16 | 보고 | 커밋하지 않음. revert 단위 정책은 사용자 결정 사항으로 보고만 함.
- 17 | 반영(step8) | notes/DECISIONS.md 10-31 항목 추가, 10-29 270k 항목 superseded 표기.
- 18 | 반영 | 실행 순서 0→10 준수. haiku-coder/haiku-checker 위임 수단 없음(Agent 도구 부재)으로 인라인 수행, 보고.
D-A: submit 없음, runTurn(repl.go)이 단일 전송 지점
D-D: effectiveThreshold에 compact-window 최소값 미적용(설계 그대로)
