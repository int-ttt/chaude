# digest: io-architecture plan + scope + research notes (261031)
task: read-only summary; no source edits
files_read: 6/6 present (plan-review, scope, subagent-context-model, prompt-cache-ttl-clear, claude-oauth-policy-and-cli, oauth-policy-primary-sources)

## (1) notes/runs/261031-plan-io-architecture-review.md
status: review (opus-architect), not executed
verdict_line: "verdict: 처음부터 재설계는 비권장. 읽기·쓰기 흐름은 현 구조(0.8.x)와 대부분 같고, 비용의 큰 몫은 출력이 아니라 Opus 컨텍스트 생성(캐시 쓰기)임."
verdict_scope: 전체 재설계 판정. TUI vs stream-json 전용 판정은 없음; 쓰기 섹션(line 14)에 포함
tui_vs_resident_child: 별도 인터랙티브 클라이언트 비권장 (line 14). reasons:
  - R1: 현 child = 상주 `claude -p --input-format stream-json` 세션, 맥락 유지
  - R2: TUI 전환 시 비용·권한·알림 신호 상실 (cites 261030-plan-interactive-child-review.md:32; not opened by me)
  - R3: Haiku 콜드스타트 약 $0.009
  - R4: 맥락 100k 초과 시 Haiku 단가 5배 (cites FACTS.md:10; not opened by me)
  - recommended: 작업 흐름별 상주 stream-json haiku-coder child (100k 전 회전); 일회성은 일반 에이전트
evidence: 실측 A 1회 Opus $0.5437 = 캐시쓰기 64% / 캐시읽기 14% / 출력 23%
미확인 (per plan):
  - caveman B·C 미완 (A만, $1 상한 중단) -> caveman 이득 미확인
  - billing.md:18 구독 5h 사용률이 API 단가 비율을 따르는지 미확인
  - 측정 6항목 도입 전 필요 (lines 30-36)
  - missing: notes/runs/261030-plan-interactive-child.md 없음
  - DECISIONS.md:43 은 caveman 항목 아님

## (2) notes/runs/261031-scope-io-architecture-branch.md
status: ESCALATE, nothing implemented
conflict: plan 쓰기 비권장 vs branch 목표 interactive-child compare
open_decision: A 비교 실험(측정)으로 한정 / B 계획 구조 + 지시문 최적화만 / C 인터랙티브 child 구현
work_items:
  1 읽기: scout/researcher 기본 4000자 유지, "줄 제한 없음" 미채택
  2 쓰기: 정확 사양 -> 상주 stream-json haiku-coder child (100k 전 회전) / 일회성 일반 에이전트; 회신 = 변경 파일·방식·검사 결과
  3 출력: Opus 영어 key: value, 최종 사용자 답만 메인 Haiku 존댓말
  4 입력: 압축 홉 없음, 대용량은 파일 경로
  5 측정 6항목 선행
files_likely: agents/*.md, CLAUDE.md, internal/harness/livechild.go, repl.go, orch*.go, notes/FACTS.md
unverified: branch uncommitted 변경 다수; 인터랙티브 child 구현 범위 미확인

## (3) research notes (claims; "unverified" = note's own label)
### subagent-context-model.md (checked 2026-10-30; src https://code.claude.com/docs/en/sub-agents, WebFetch summary)
- 일반 subagent는 새 격리 컨텍스트, 부모 대화 이력 미상속 (verified via WebFetch)
- 시스템 프롬프트 = 에이전트 자체 프롬프트 + 기본 환경 정보 (Claude Code 시스템 프롬프트 아님); + CLAUDE.md, git status 스냅샷
- model 우선순위: per-invocation > frontmatter > CLAUDE_CODE_SUBAGENT_MODEL > 메인 모델; Agent 호출 시 alias만 허용
- fork는 부모 대화 상속 (fork 본문 미확인, unverified)
- "subagent의 subagent 호출 불가": third-party snippet only, unverified
- REPL vs -p 토큰/시스템 프롬프트 차이는 이 파일에 없음

### prompt-cache-ttl-clear.md (asof 2026-10-30)
- API 기본 TTL 5분; 1h opt-in, write 2x; 5m write 1.25x; read 0.1x (Opus 5.5/Sonnet 5.5 0.05x). src: platform.claude.com/docs/en/build-with-claude/prompt-caching
- Claude Code main conversation: 1h은 subscription 플랜 내에서만, 5m은 usage credits/API key/cloud. src: code.claude.com/docs/en/prompt-caching.md
- 공유 규칙: model, speed, prefix 일치 시 공유; 같은 디렉터리 병렬 세션은 캐시 공유 가능; scope = 한 머신+디렉터리
- /clear: "Start a new conversation with empty context"; 서버 캐시 영향은 NOT STATED (inference only, unverified)
- -p vs 인터랙티브 캐시 공유: 문서에 직접 언급 없음 (미확인)
- doubts: API 페이지 chars 100k-165k 미독; Claude Code 5m/1h 구분의 API 사용자 적용 미확인

### claude-oauth-policy-and-cli.md (2026-10-31)
- Q1-a 공식: "OAuth authentication is intended exclusively for purchasers of Claude Free, Pro, Max, Ultra, Team, and Enterprise subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications." (code.claude.com/docs/en/legal-and-compliance)
- Q1-b: Agent SDK 개발자는 API key 사용; 구독 자격증명으로 제3자 요청 라우팅 금지; 자격증명 수집·중개 금지
- Q1-c: 최종 사용자가 수정되지 않은 바이너리에 본인 구독으로 로그인하는 것은 허용
- Q1-e: support article: 구독 로그인 시 Agent SDK·claude -p도 구독 한도 차감; "Update June 15, 2026: We've paused the previously-announced changes to Claude Agent SDK usage."
- Q1-f: "...including the Agent SDK ... is not permitted" 문구는 2차(WinBuzzer/Register) only, 현 페이지 본문에 없음
- Q4: 서버측 차단 보고 전부 2차/사용자 보고; 공식 발표 미확인; 2026-04-04 third-party harness는 extra usage 과금 (2차, InfoWorld)
- PTY/화면 스크래핑: 4개 파일 어디에도 언급 없음 (not found)
- Q3: --system-prompt 등 5개 플래그 두 모드 동작 (섹션 문구 기준); --allowedTools 미확인

### oauth-policy-primary-sources.md (2026-10-31)
- legal page: "Anthropic reserves the right to take measures to enforce these restrictions and may do so without prior notice." 탐지 메커니즘 공식 설명 NOT FOUND
- 반대 근거: authentication 페이지 "generate a one-year OAuth token with `claude setup-token`" 대상 "CI pipelines, scripts..."; Pro/Max/Ultra/Team/Enterprise 필요 (primary, re-verified)
- Agent SDK overview/quickstart NOTE: "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products..." (fetched, not re-verified)
- Consumer Terms: 영문 미확보; 재fetch 요약(인용 아님): Section 3이 API key 또는 명시 허용 없는 automated/bot/script 접근 금지 — summary only
- tension: 본인 스크립트가 본인 구독으로 Agent SDK/-p 호출하는 것의 허용 여부를 명시하는 공식 문장 없음 (file's own note)
- GitHub issue #11 "Max + Agent SDK permitted" 주장: disputed, unverified
- 2차: 2026-01-09 spoofing safeguards (Thariq); 2026-02-19 "docs clean up"
- 미확인: 탐지 헤더 메커니즘, Commercial Terms 미fetch
