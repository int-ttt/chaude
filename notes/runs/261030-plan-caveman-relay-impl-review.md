# 검토: 261030-plan-caveman-relay-impl.md
date: 261030
reviewer: opus-architect
scope: 개선점만. 계획 실행 안 함
verdict: 방향은 타당. 단 그대로 실행하면 (1) 차단 훅이 실행 중 세션·코딩 작업을 막고 (2) 튜닝이 1과제·1회 측정이라 노이즈에 맞춰지며 (3) 브랜치 명령이 미커밋 변경을 섞음. 아래 반영 후 실행 권고

## A. 빠진 단계·충돌 (우선순위 순)
- A1 CLAUDE.md:4 "Korean, 존댓말." 전역 규칙: 서브에이전트에도 로드됨. 에이전트 파일만 바꾸면 규칙 충돌 → 2.1에 CLAUDE.md:4 문구 변경 단계 추가. 예: "사용자가 직접 읽는 출력(메인 스레드)만 존댓말. 서브에이전트 간 출력은 존댓말 불필요"
- A2 notes/ 읽기를 지시하는 에이전트 규칙 제거 단계 누락: opus-architect.md, sonnet-analyst.md 등 "haiku-scout로 notes/FACTS.md, DECISIONS.md, LESSONS.md 읽기" 지시 + importclaude.go:156 "Re-read notes/". 훅이 deny하면 에이전트가 재시도 반복 → grep "FACTS.md|DECISIONS.md|LESSONS.md" .claude/agents CLAUDE.md internal/ 결과 전부를 변경 목록 1.6으로 추가
- A3 허용 목록 범위 오류 위험: 2.2가 "입력 경로가 허용 목록 밖이면 deny"면 소스 코드 Read도 막힘. Claude Code Edit은 사전 Read 필요 → haiku-coder 작업 불가. 정책을 "기억·문서 경로 denylist + 예외 allowlist"로 변경
  - deny 대상: notes/**, docs/**, imports/**, ~/.claude/projects/*/memory/**, ~/.claude/CLAUDE.md, CLAUDE.local.md, 루트의 *.md 중 allowlist 외
  - 항상 허용: 소스 코드(*.go, *.py 등), .claude/agents/*.md, allowlist 파일 항목
  - Write/Edit는 차단 안 함 (ledger.py의 LEDGER.md 쓰기, notes/runs/ 결과 저장 유지). 단 Edit 전 Read가 막히는 notes/ 파일은 Write로 append하는 기존 방식 유지 확인
- A4 1.3 누락 지점: Claude Code 자동 메모리(~/.claude/projects/<proj>/memory/), 사용자 전역 ~/.claude/CLAUDE.md, /resume·import 스냅샷 주입(importclaude.go, handoff.go withReco), UserPromptSubmit 훅의 notes/pastes 경로 안내. 각각 차단/유지 결정 칸 추가
- A5 실행 중 세션 보호: .claude/settings.json·hooks는 현재 작업 트리에서 즉시 적용됨. master 작업 트리에서 훅을 추가하면 지금 세션(하네스 자체)도 차단됨 → 구현·테스트는 별도 git worktree에서만 (C절)
- A6 단계적 적용: 훅을 먼저 log-only 모드(deny 대신 notes/metrics 또는 stderr에 would-deny 기록)로 1회 세션 운용 → 오탐 0 확인 후 enforce. 환경변수 CAVEMAN_GUARD=log|enforce|off, 기본 log
- A7 Bash 경로 탐지 한계 명시: cat/rg/sed/head, 상대경로, ../, 심볼릭 링크, 변수 확장 우회 가능. "best-effort" 로 계획서에 명시하고 4.2에 우회 케이스 포함
- A8 아카이브 경로 충돌: notes/archive/261030/ 이미 존재(untracked). 원본 보존은 git 브랜치로 충분. 굳이 복사 시 notes/archive/261030/caveman-relay-orig/ 하위로
- A9 에이전트 배정 명시: 훅·Go 수정 = haiku-coder, 정적 검사 = haiku-checker, 튜닝 루프 소유 = haiku-prompter, 병합 전 diff 검토 = sonnet-reviewer
- A10 단계 분리: 1단계 = 존댓말 해제 + 차단 훅 + 초기 맥락 목록. 2단계 = caveman 릴레이(읽기 흐름) + 튜닝. U7은 2단계만 막으므로 1단계 진행 가능

## B. 6절 미확정 10건 권고 기본값
- U1 TUI 출력 haiku: 기본값 = 메인 스레드(agent_id 없음)만 존댓말 유지. 서브에이전트 handback은 메인이 재작성하므로 존댓말 불필요. haiku-prompter의 라운드 로그가 TUI에 그대로 표시되면 예외 포함. 확인: grep -rn "haiku" internal/harness/tui.go repl.go render*.go
- U2 초기 맥락 지정: 기본값 = .claude/context-allow.txt (한 줄 1경로, 저장소 기준 상대경로, glob 허용, # 주석). 훅과 chaude가 같은 파일을 읽는 단일 원천. CLI --context <file>는 덮어쓰기용으로 나중에 추가. settings.json은 Claude Code 스키마와 섞여 비권고
- U3 CLAUDE.md 자동 로드: 기본값 = 차단하지 않음(사용자 규칙 원천, 라우팅 필수). 대신 A1대로 문구만 수정. allowlist에 CLAUDE.md 고정 포함
- U4 classifier.go 문구: 기본값 = 1단계에서 변경 안 함. 라우터 출력은 분류 결과라 사용자 노출 아님. 바꿀 경우 문구와 sha256 기대값을 같은 커밋에서 갱신
- U5 브리프 내 notes/ 재읽기 금지: 기본값 = 별도 강제 불필요. 훅이 읽기 자체를 deny하므로 지시가 있어도 효과 없음. A2의 지시 제거 + deny 로그 건수 집계로 대체. 샘플 검사는 폐기
- U6 튜닝 상한: 기본값 = 최대 5회 + 누적 토큰 ≤ 기준(A조건 1회) 토큰 × 8 (기준선 2회 + 루프 5회 + 홀드아웃 1회 분량). 금액 상한은 사용자 결정
- U7 참조 read 계획서 10절 제약: 기본값 = 1단계에서는 개정 안 함. 2단계 착수 시 재검토
- U8 임계값 30%: 기본값 = 유지. 단 기준선 노이즈(D2)보다 클 때만 유효. 노이즈가 15% 넘으면 30% 판정 보류 후 사용자 보고
- U9 FACTS/DECISIONS 존댓말 기록: 사용자 결정 아님. 실행 전 haiku-scout로 grep -n "존댓말" notes/FACTS.md notes/DECISIONS.md 결과만 계획서에 기입. 기록이 있으면 같은 단계에서 문구 갱신
- U10 미커밋 변경: C절 참조. 기본값 = 사용자 승인 후 master에 WIP 커밋 → 그 커밋에서 worktree 분기
- 결론: 사용자만 답할 질문은 U1 확인, U6 금액, U10 커밋 승인 3건으로 축소. 7절 "U1~U10 응답"을 "U1·U6·U10 응답 + 나머지 기본값 적용"으로 수정

## C. 브랜치 분리 시 master 미커밋 변경 처리
- 문제1: 3절 "main 기준 분기"와 `git checkout -b`는 모순. checkout -b는 현재 master의 미커밋 변경을 그대로 새 브랜치로 가져가고, main 기준도 아님
- 문제2: 변경 대상(.claude/agents/*.md, hooks/*.py, settings.json, repl.go 등) 대부분이 이미 미커밋 수정 상태. main 또는 HEAD에서 깨끗하게 분기하면 0.8.2 작업 내용이 빠져 계획의 줄 번호·내용이 모두 어긋남
- 문제3: stash는 untracked 다수(orch.go, turnspend.go, testdata/, imports/, notes/...)와 섞여 복구 위험. 사용 금지
- 권고 절차 (실행은 사용자 승인 후):
  1. 기준선 기록: git status --porcelain > notes/runs/261030-wip-status.txt, go test ./internal/harness/... 결과 저장 (D4)
  2. master에서 WIP 커밋: .DS_Store 제외, notes/metrics 포함 여부는 사용자 확인. 메시지 예: "0.8.2-wip: orch, usagepoll, turnspend (pre caveman-relay)"
  3. git worktree add ../claude-harness-caveman -b caveman-relay-impl master
  4. 모든 구현·훅·튜닝은 ../claude-harness-caveman 에서만. 현재 작업 트리와 실행 중 세션은 그대로
  5. 병합은 sonnet-reviewer 검토 후 사용자 승인으로. PR 대상 브랜치(main vs master) 사용자 확인
- 대안(커밋 거부 시): worktree를 master HEAD에서 만들고 필요한 미커밋 파일만 복사. 단 두 트리 동기화 부담 큼 → 비권고

## D. 자동 튜닝 루프 종료 조건·과적합 방지
- D1 튜닝 변수에서 "허용 문서 목록 항목" 제거. 지표를 올리려고 문서를 추가하는 것은 차단 목적을 깨는 보상 해킹이며 5절의 "허용 목록 정책 변경 금지"와도 충돌. 변수는 caveman 스타일 문구, 번역 프롬프트 문구 2개만
- D2 노이즈 측정: 루프 시작 전 A조건 2회, B조건(초기안) 2회 실행. 지표별 |run1-run2|/평균 = 노이즈 폭. "개선" = 노이즈 폭 초과 향상만 인정
- D3 평가셋 고정: 튜닝 과제 T1 1개 + 홀드아웃 과제 T2 1개를 루프 시작 전 확정하고 지표·채점 기준도 동결. 루프 중 T2 실행 금지
- D4 게이트: 매 루프 4.1~4.3 + 번역 검사(참조 9절) PASS가 전제. 실패 시 그 루프는 무효로 되돌리고 (c) 카운트에 넣지 않음. 무효 2회 연속이면 중단
- D5 종료 조건 수정안 (하나라도 충족 시 종료):
  - (a) T1에서 "유망" 기준 충족 → 즉시 T2 1회 검증. T2도 충족 시 성공 종료. T2 미충족 시 "과적합" 사유로 종료하고 최고안 보고
  - (b) 최대 5회
  - (c) 연속 2회 노이즈 폭 이내 또는 악화
  - (d) 누적 토큰 상한(U6)
  - (e) enforce 모드에서 정상 작업 오탐 deny 1건 이상 → 즉시 중단(안전)
- D6 채택안 = 마지막 안이 아니라 지표 최고안. 각 루프 diff와 지표를 notes/runs/261030-result-caveman-relay-impl-<n>.md에 기록, 종료 시 최고안으로 복원
- D7 변경 1개/루프 유지. 문구 길이 증가 상한(예: +20%) 설정해 지시문 비대화 방지

## E. 테스트 범위 보강
- E1 4.1 기준 수정: "존댓말 잔존 0건"이 아니라 파일별 기대 상태 표로 검사 (TUI 출력 대상 = 존댓말 유지, 나머지 = "Korean (no honorifics)" 존재). CLAUDE.md:4 포함
- E2 4.2 훅 케이스 최소 목록 (입력 JSON을 훅에 파이프, exit code·decision 확인, python3 -I 실행):
  - Read notes/FACTS.md → deny / Read internal/harness/repl.go → allow / Read CLAUDE.md → allow
  - Read 절대경로·상대경로·../ 경유 notes 경로 → deny
  - Grep path 생략(cwd 전체 검색) → notes/ 제외 처리 또는 deny 확인
  - Glob "notes/**" → deny
  - Bash "cat notes/LESSONS.md", "rg x notes/" → deny / Bash "go test ./..." → allow
  - Read ~/.claude/projects/*/memory/x.md → deny
  - WebFetch → 정책 결정값대로 (외부 문서 차단 여부를 계획서에 명시)
  - Write notes/runs/x.md → allow / Edit 소스 파일 → allow
  - agent_id 있음·없음 각각 동일 결과
  - 잘못된 JSON·allowlist 파일 없음 → 정의된 기본 동작(권고: allowlist 없으면 denylist만 적용, 훅 오류 시 allow + stderr 경고)
  - CAVEMAN_GUARD=log 일 때 deny 대신 기록만
- E3 4.3 Go 테스트: 변경 전 기준선 결과를 먼저 저장. 미커밋 테스트 파일(orch_test, usagepoll_test 등)이 이미 실패할 수 있으므로 회귀 판정은 기준선 대비로
- E4 회귀 테스트 추가: main-thread-guard.py 기존 동작 유지, ledger.py가 LEDGER.md 계속 기록, session-context.py SessionStart 출력에 notes 내용 미포함 확인, style-adapter.py [inbox] 태그 미주입 확인
- E5 통합 테스트는 worktree에서만 실행. 현재 세션 설정 파일 무변경을 git diff --stat으로 확인

## F. 7절 완료 기준 수정안
- U1·U6·U10 사용자 응답, 나머지 B절 기본값 기록
- WIP 커밋 + worktree caveman-relay-impl 생성
- 4.1~4.3 + E4 PASS, log-only 1세션 오탐 0
- 튜닝 결과 파일 + 종료 사유 1개 + T2 홀드아웃 결과
- 바꾸면 안 되는 것: 참조 계획서 3개, 현재 master 작업 트리의 .claude/settings.json·hooks(병합 전), classifier.go(1단계)
