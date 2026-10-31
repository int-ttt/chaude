# review: 261031-plan-model-call-policy (개선안만, 실행 금지)
대상: notes/runs/261031-plan-model-call-policy.md
형식: 번호 | 대상 | 개선 내용 | 이유

1 | D2/Q1 | repl.go:505 `r.sess.Send(text)` 앞의 route() 호출을 없애고 입력은 항상 현재 세션 Send로 직행. "haiku→opus 전달"은 Go가 아니라 haiku 메인 세션의 Agent 위임(opus-architect, frontmatter model: opus)으로 구현. Go에 opus 1회 호출 래퍼(claude -p)나 haiku 출력 파싱 후 opus 세션 재전송은 두지 않음. | Go 쪽 전달 로직은 분류기 재도입과 같고 세션 연속성·REPL 출력 형태를 깨뜨림.
2 | D2 | 단일 지점 이름 확정: route() 대체 `func (r *Repl) submit(text string)` = 예산 게이트 + r.spend 리셋 + r.sess.Send. 호출부 1곳. | 구현자가 다시 정하지 않도록.
3 | D2 | 인수 조건 추가: "haiku 메인이 opus에 전달"하라는 지시가 있는 위치(CLAUDE.md / .claude/agents / plan/caveman-structure.md)를 확인. 없으면 범위 밖 후속 항목으로 표시. | Go 변경만으로는 전달 동작이 보장되지 않음.
4 | D1 | classifier.go 삭제 심볼 명시: classify, route, parseRouteFull, routerModelDefault, routerModelEnv, routerModel(), 분류 프롬프트 상수(classifyPromptSonnet 등), [routing: ...] 힌트 상수, classifyFn. 카테고리 rotation(repl.go:942-946, category_test)이 쓰는 심볼은 남기고 파일명 유지. | 파일 통째 삭제 시 카테고리 rotation 빌드 깨짐.
5 | D1 | 착수 전 grep "parked|switchSession" 사용처 확인 단계 추가. switchSession은 전환 UI용으로 유지, parked가 route 전용이면 삭제. | 전환 UI 유지(사용자 결정)와 죽은 코드 정리를 함께 만족.
6 | D3 | 상수 `haikuThreshold = 80_000`, `opusThreshold = 200_000`. -threshold 플래그 기본값(repl.go:1068) 150_000→haikuThreshold, 플래그는 haiku 전용 덮어쓰기로 유지. thresholdFor: opus→opusThreshold, 그 외(haiku·빈 값·미지 값)→r.a.Threshold. repl.go:27-28 주석 갱신. | 플래그 기본값을 안 바꾸면 haiku는 계속 150k.
7 | D3 | thresholdFor의 판정을 `model == "opus"` 대신 strings.Contains(model, "opus")로. | 전체 ID("claude-opus-5-5")가 오면 opus가 80k에서 조기 rotation.
8 | D3/(b) | repl.go:951 조건 `autoOK && cooled && lastCtx>=thr`는 수정 불필요(thr만 바뀜). `>=`이므로 80,000/200,000에서 발생, 79,999/199,999 미발생 → 5절 경계 테스트와 일치. effectiveThreshold()가 compact-window(400k)와 min을 취하는지 확인 항목 추가(200k<400k라 영향 없음 예상). 카테고리 rotation(942-946)은 그대로. | 조건식은 맞고 임계 출처만 바꾸면 됨.
9 | D3 위험 | opus 5.5 컨텍스트 창과 Claude Code 자동 압축 시점이 200k 이후인지 확인 항목 추가. | 압축이 먼저 일어나면 lastCtx>=200k에 도달하지 못해 rotation이 안 걸림.
10 | D3 | 저장된 세션 상태(model="sonnet")로 재개하면 haiku 임계 80k가 적용됨을 명시하고 테스트 1건. | sonnet 분기 제거의 경계 사례, 재개 직후 rotation 가능.
11 | Q3/(c) | haikupool.go 유지: haikuWorker, workerPool, newWorkerPool, noteEdit(repl.go:576), has. 삭제: candidates, candidateText, maxCandidates(사용처 classifier.go:183,224 라우터 전용). has()에 운영 호출처가 없으면 TestWorkerPool용 읽기 API로 둠. 라우터 제거 후 풀은 쓰기 전용 상태가 됨을 계획서에 기록. | 풀 유지(사용자 결정)를 지키면서 라우터 심볼만 제거.
12 | D4/Q2/(d) | settings.json "model": "opus"→"haiku". REPL은 core.go:146에서 --model을 직접 넘기므로 settings.json은 저장소에서 claude를 직접 켤 때만 적용됨 → "메인=haiku"와 맞춤. 6행 CLAUDE_CODE_SUBAGENT_MODEL은 건드리지 않음. 확인 항목: opus-architect가 frontmatter 모델 opus로 실행되는지(기본 서브에이전트 모델보다 우선). internal/settings/settings.go:251이 이 파일을 원본으로 읽으므로 internal/settings 테스트도 실행. | 메인 정의 일관 + 위임 대상이 실제 opus인지 보장.
13 | D5 | orch.go orchDefaultModel에 sonnet 없음(b·c·d·f=opus, e=haiku) → "변경 없음, 확인만"으로 수정. 빈 값 모드는 -model 기본 haiku로 귀결됨을 명시. | 불필요한 수정 방지.
14 | D6/(e) | 테스트 범위 고정. 삭제: routerprompt_test.go, classifier_model_test.go, multiproc_s_test.go TestRoute*(268-410)·classifyFn 대체(371). 재작성: conductor_test.go TestMainRouteSwitchPolicy(91)→"분류 없이 현재 세션 Send". 수정: TestWorkerPool(123,194-210)에서 candidates 단언 제거. 신규: TestThresholdFor(haiku 80k, opus 200k, claude-opus-5-5 200k, 빈 값 80k, sonnet 80k) + rotate 경계 4건(TestIdleRotateKeepsPendingSnapshot 방식). 확인만: orchws_test TestWSNeedsRotateCases, category_test, resumeshift_test, acceptance_test(라이브)에서 150000|150_000|sonnet 리터럴 grep. | 누락 없이 파일·함수 단위로 고정.
15 | 5절 검증 | "sonnet 잔존 0건" grep 범위를 internal/harness/{repl,classifier,orch,haikupool}.go와 그 테스트로 제한. budget.go 별칭(FACTS:11), .claude/agents/sonnet-*는 범위 밖. go vet ./... 추가. | 저장소 전체 grep은 범위 밖 수정을 유도. 삭제 후 미사용 import 확인.
16 | Q4 | git revert로 충분. 조건: 구현을 단일 커밋(또는 연속 커밋 + 착수 전 태그 pre-model-policy)으로 하고, 작업트리의 무관 변경(notes/metrics, imports/, .DS_Store)은 별도 커밋 또는 제외. settings.json은 추적 파일이라 revert에 포함. CHAUDE_ROUTER_MODEL은 제거 후 오류 없이 무시됨을 기록. | revert 단위 오염 방지.
17 | 문서 | notes/DECISIONS.md:41(10-29: haiku 80k, opus·sonnet 270k)과 이번 결정(opus 200k)이 다름 → 구현 단계에 DECISIONS.md 10-31 항목 추가(270k 대체) 넣기. | 이후 에이전트가 옛 270k를 사실로 쓸 위험.
18 | 순서·담당 | 권장 순서: (1) parked/has 사용처 grep (2) 라우터 테스트 삭제 (3) classifier 라우터 심볼 삭제 + submit 대체 (4) 임계 상수·플래그 (5) haikupool 정리 (6) 신규 테스트 (7) settings.json (8) DECISIONS.md (9) 검증. 단계마다 go build. Go 변경은 haiku-coder 한 인스턴스로 연속, 검증은 haiku-checker. | 빌드가 깨진 중간 상태 최소화.

## 충돌
없음. (17번의 270k는 이전 결정이며, 사용자 결정 opus 200k가 더 최신이라 그대로 따름.)

## 사용자 질문
- "sonnet 계층 없음"이 .claude/agents의 sonnet-analyst·sonnet-reviewer까지 포함하는지 (현재 개선안은 범위 밖으로 둠).
