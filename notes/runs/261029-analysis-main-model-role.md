# main model role (261029)
user_q: 메인 모델이 왜 있는지(역할/존재 이유), 직전 다중 클라이언트 질문과 연결
conclusion: chaude 메인 = 항상 Haiku 상주 세션(`claude -p stream-json --model haiku`). 일은 안 하고 "듣기 + 위임 + 최종 한 번 답변"만 담당. 이유 = 토큰/비용 절감 + 세션 연속성(대화 맥락 보유).
evidence:
- CLAUDE.md:2,4 "main thread listens and routes (Haiku in chaude; Opus only if forced); subagents do all work"; 메인은 작업 도구 없음(hook 강제), 메인 토큰 최소화.
- CLAUDE.md:5,16 흐름 = 대기 -> 프롬프트 -> 한 번의 위임 라운드(모든 agent 호출 한 메시지, foreground) -> 최종 답 1회. 테스트/빌드/편집은 Haiku/Sonnet agent 몫.
- CLAUDE.md:32 conductor: 메인은 항상 Haiku(`--model haiku`), classifier는 Sonnet이 크기 태그만. large/설계 -> Haiku가 plan 작성 -> opus-architect/sonnet-analyst 리뷰 -> Haiku가 Haiku agent로 실행 -> sonnet-reviewer. 강제 `opus로/sonnet으로`만 예외.
- repl.go:993 `--model` 기본값 "haiku"; repl.go:1010 `--conductor` 기본 true.
- classifier.go:85-101 pickModel/pickTarget: 메인은 항상 Haiku(강제 Sonnet/Opus 유지). classifier.go:72-81 delegateHint/sonnetHint/planHint = 메인 프롬프트에 붙는 "[routing: ...]" 위임 지시.
- classifier.go:180-212 classify = Sonnet 1회성 짧은 호출로 티어 태그 + 간결 task; route()가 메인에 보낼 텍스트 결정.
- repl.go:27-28,44 회전: Haiku 메인 80k(-threshold), Opus/Sonnet 270k -> 새 세션 + handoff snapshot (repl.go:699 rotate). 메인이 가벼워서 자주 교체해도 싸다.
- repl.go:162-163 pool/parked: 모델별 세션 id를 parked로 보관해 전환 시 resume.
relation_to_concurrency (261029-analysis-concurrency-limit.md):
- 메인 = chaude가 소유한 상주 child 중 "현재 활성" 하나(해당 cwd+model). children.go:16 childKey(cwd,model), children.go:81 Park로 다른 모델 child 보관. 따라서 한 chaude 안의 동시 claude 프로세스 = 활성 메인 + parked(모델별) 정도, 개수 상한 코드는 없음(MaxWorkers 미사용, dispatch.go:11-30).
- 서브에이전트(opus-architect 등)는 chaude 프로세스가 아니라 메인 claude 세션 내부 Agent 호출이라 별도 chaude child가 아님 (inference: CLAUDE.md:32 흐름 기준, 프로세스 구조는 미검증).
- 여러 chaude를 동시에 띄우면 각자 메인 Haiku + 자기 children을 가짐 (dispatch.go:61-83 OwnerPID 검사). 직전 ps에서 보인 haiku 2개는 한 chaude의 활성/교체 중 child로 추정(미검증).
uncertainty: hook으로 도구 제한이 실제 강제되는지는 .claude 설정 미확인(CLAUDE.md 서술 기반). 서브에이전트 프로세스 구조는 추론.
discrepancy: CLAUDE.md:32 says classifier is always Sonnet, but classifier.go:15 routerModelDefault="haiku" (sonnet only via CHAUDE_ROUTER_MODEL=sonnet, classifier.go:45-48). Code wins; doc is stale.
extra_client: classify() (classifier.go:185-188) spawns a separate short-lived `claude -p --model <router> --tools "" --no-session-persistence` in os.TempDir per prompt -> one more transient claude process besides main/children.
