# 구현 계획서: caveman 릴레이 (존댓말 제한 해제 + 메모리·외부 문서 차단 + 자동 튜닝)
date: 261030
status: 계획만 작성. 코드·설정 수정 없음. 브랜치 생성 전
merged_from: notes/runs/261030-plan-caveman-relay-impl-review.md (개선점 병합 항목 1~6)
original_archived: notes/archive/261030/caveman-relay-orig/261030-plan-caveman-relay-impl.md
refs_read_only: notes/runs/261030-plan-caveman-relay.md, 261030-plan-caveman-relay-read.md, 261030-plan-caveman-relay-flow.md (수정 금지, 읽기만)
stage: 1단계 = 존댓말 해제 + 차단 훅(log→enforce) + 초기 맥락 목록 / 2단계 = caveman 릴레이(읽기 흐름) + 튜닝

## 0. 요청 요지
item_1: (a) TUI 출력용 haiku를 제외한 모든 모델의 존댓말 제한 해제 (b) 모든 모델이 메모리 또는 기타 문서에서 맥락을 가져오는 것 차단 (c) chaude가 지정한 문서만 초기 맥락으로 허용
item_2: 반복 테스트로 자동 튜닝
item_3: 별도 브랜치에 구현 후 테스트
extra: 입력·읽기 결과 caveman화 (참조 3개 계획서 범위)

## 1. 대상 파일 목록 (grep 확인 경로만)

### 1.1 존댓말 규칙 (.claude/agents/*.md, 전역 규칙)
grep: `grep -rn "존댓말" .claude/agents/*.md CLAUDE.md`
- .claude/agents/haiku-coder.md:16 (Final message, Korean 존댓말)
- .claude/agents/haiku-drafter.md:19 (Final message)
- .claude/agents/haiku-batch-worker.md:15 (Final message)
- .claude/agents/haiku-researcher.md:15 (Final message)
- .claude/agents/haiku-runner.md:16 (Final message)
- .claude/agents/haiku-extractor.md:23 (Final message)
- .claude/agents/haiku-checker.md:14 (Final message)
- .claude/agents/haiku-scout.md:16 (Final message)
- .claude/agents/haiku-prompter.md:25 (handback, `## 최종 답변` 등 한국어 형식 포함)
- .claude/agents/sonnet-reviewer.md:14 (Final message 요약 한국어)
- .claude/agents/sonnet-analyst.md:11 (사용자용 답변 초안), :16 (Final message)
- .claude/agents/opus-architect.md:16 (사용자가 읽는 내용), :17 (Final message)
- CLAUDE.md:4 ("Korean, 존댓말." 전역 규칙. 서브에이전트에도 로드됨 → 에이전트 파일만 바꾸면 충돌. 문구 변경 필수)
  - 변경안: "사용자가 직접 읽는 출력(메인 스레드)만 존댓말. 서브에이전트 간 출력은 존댓말 불필요"

### 1.2 존댓말 규칙 외 연관 (목록만)
- internal/harness/classifier.go:20, :31 — 라우터 프롬프트 "no honorifics ... reply: Korean 존댓말". "keep this text byte-identical (sha256 test)" 주석 (classifier.go:18). 1단계 변경 안 함 (U4). 변경 시 문구와 sha256 기대값을 같은 커밋에서 갱신
- notes/FACTS.md, notes/DECISIONS.md — 존댓말 기록 여부 미확인. 실행 전 `grep -n "존댓말" notes/FACTS.md notes/DECISIONS.md` 결과를 여기 기입 (U9). 기록이 있으면 같은 단계에서 문구 갱신

### 1.3 메모리·외부 문서 접근 지점 및 차단 결정
차단 지점 (기존 + 추가 항목 포함):
| 지점 | 내용 | 결정 |
|---|---|---|
| .claude/hooks/session-context.py:115 | `head("notes/OPEN.md", 20)` SessionStart 주입 | 차단 (allowlist 외) |
| .claude/hooks/session-context.py:118 | `tail("notes/LEDGER.md", 5)` | 차단 (allowlist 외) |
| .claude/hooks/session-context.py:154 | `tail("notes/INBOX.md", 50)` | 차단 (allowlist 외) |
| .claude/hooks/style-adapter.py:28, :176 | `notes/INBOX.md` → `[inbox]` 태그 주입 | 차단 |
| .claude/hooks/style-adapter.py:182-187 | `notes/pastes/<hash>.md` 경로 안내 | 차단 |
| .claude/hooks/ledger.py:85 | `.claude/agents/<kind>.md` 읽기 | 유지 (소스 정책 밖, 항상 허용) |
| .claude/hooks/ledger.py:97 | `notes/LEDGER.md` 쓰기 | 유지 (Write 차단 안 함) |
| .claude/hooks/main-thread-guard.py:8, :24 | WORK_TOOLS, agent_id 있으면 통과 | 참고 (신규 훅은 agent_id 무관 적용) |
| .claude/hooks/agent-model-pin.py:4 | 모델 pin | 참고만 (차단 아님) |
| internal/harness/importclaude.go:156 | "Re-read notes/ (FACTS.md, DECISIONS.md) for durable state." | 문구 제거 (재독 지시 제거) |
| internal/harness/handoff.go:74 | withReco 이전 추천 주입 (메모리성 맥락) | 차단 대상. 1단계에서 주입 경로 grep 확인 후 확정 |
| 자동 메모리 `~/.claude/projects/<proj>/memory/**` | Claude Code 자동 메모리 | 차단 (훅 deny) |
| 사용자 전역 `~/.claude/CLAUDE.md` | 전역 규칙 문서 | 차단 (훅 deny) |
| /resume·import 스냅샷 주입 (importclaude.go, handoff.go) | 세션 재개 시 과거 맥락 주입 | 차단 지점으로 추가. 주입 경로는 1단계에서 확인 후 확정. 훅으로 못 막는 경로는 would-deny 로그로만 집계 |
| CLAUDE.md (repo) | 사용자 규칙 원천, 라우팅 필수 | 비차단 (U3). 문구만 1.1대로 수정. allowlist에 고정 포함 |
| internal/harness/repl.go:252 `--append-system-prompt` | rule 주입 | 유지 (맥락 문서 아님) |
| internal/harness/core.go:146 `claude -p ... --model` | 자식 호출 인자 | 유지. 초기 맥락 지정 지점 (1.4) |

### 1.4 chaude 초기 컨텍스트 지정 (U2 기본값 채택)
- 단일 파일: `.claude/context-allow.txt`. 한 줄 1경로, 저장소 기준 상대경로, glob 허용, `#` 주석
- 훅(allowlist 판정)과 chaude(초기 주입)가 같은 파일을 읽는 단일 원천
- CLI `--context <file>` 덮어쓰기는 나중에 추가. settings.json은 Claude Code 스키마와 섞이므로 비권고
- grep 근거: "initial|--file|@\S+\.md|prepend|seed", "add-dir", "--settings" 결과 chaude 전용 초기 문서 플래그 미발견

### 1.5 TUI 출력용 haiku (U1, 사용자 응답 대기)
- 기본값(응답 전까지 임시): 메인 스레드(agent_id 없음)만 존댓말 유지. 서브에이전트 handback은 메인이 재작성하므로 존댓말 불필요
- haiku-prompter 라운드 로그가 TUI에 그대로 표시되면 예외 포함
- 확인: `grep -rn "haiku" internal/harness/tui.go internal/harness/repl.go internal/harness/render*.go`

## 2. 구현 방식 (안)
- 2.1 존댓말 해제: 1.1 목록의 "Korean 존댓말"을 "Korean (no honorifics)"로 교체. TUI 출력 대상(U1)은 예외. 대상 확정 전 변경 금지
- 2.2 차단 훅: 신규 `.claude/hooks/context-allowlist.py` (PreToolUse, Read/Grep/Glob/Bash/WebFetch/WebSearch)
  - 정책: denylist + 예외 allowlist (허용 목록 밖 전체 차단 방식은 쓰지 않음. 소스 Read가 막히면 haiku-coder 작업 불가)
  - deny 대상: `notes/**`, `docs/**`, `imports/**`, `~/.claude/projects/*/memory/**`, `~/.claude/CLAUDE.md`, `CLAUDE.local.md`, 루트 `*.md` 중 allowlist 외
  - 항상 허용: 소스 코드(*.go, *.py 등), `.claude/agents/*.md`, `CLAUDE.md`, allowlist 항목
  - Write/Edit는 차단 안 함 (notes/runs/ 결과 저장, LEDGER.md 기록 유지)
  - 모든 agent_id(메인·서브) 동일 적용
  - Bash·Grep 경로 탐지는 best-effort. 우회 가능: cat/rg/sed/head 등 명령 변형, 상대경로, `../`, 심볼릭 링크, 변수 확장. 4.2에 우회 케이스 포함
  - 모드: `CAVEMAN_GUARD=log|enforce|off`, 기본 `log`. log 모드는 deny 대신 would-deny를 notes/metrics 또는 stderr에 기록
  - 적용 순서 (item 1): (a) log 모드로 1세션 운용 → would-deny 오탐 0 확인 → (b) enforce 전환. 기본 동작은 log
  - 소스 읽기는 항상 허용. 차단은 메모리·외부 문서 경로만
  - 오류 기본 동작: 훅 예외 → allow + stderr 경고. allowlist 파일 없음 → denylist만 적용
- 2.3 초기 맥락: `.claude/context-allow.txt` 한 파일로 관리. chaude 주입 + 훅 allowlist 공용 (1.4)
- 2.4 session-context.py, style-adapter.py의 notes 주입: allowlist 기준으로 축소 또는 차단 (1.3 표)
- 2.5 notes 재독 지시 제거 (item 2):
  - 훅으로 브리프 텍스트는 차단 불가 → 지시 자체를 제거
  - 대상 검사: `grep -rn "FACTS.md\|DECISIONS.md\|LESSONS.md" .claude/agents CLAUDE.md internal/` 결과 전부 제거 (importclaude.go:156 포함)
  - 별도 샘플 검사·강제 수단은 두지 않음 (U5). deny 로그 건수 집계로 대체
- 2.6 caveman 릴레이 (2단계): 참조 계획서(2절 형식, 9절 번역 검사) 사용. 읽기 흐름은 참조 read 계획서 2.6 스키마. U7은 2단계 착수 시 재검토 (1단계 보류)

## 3. 브랜치
- 명령 (실행 승인 후): `git worktree add -b caveman-relay-impl ../claude-harness-caveman master`
- stash 금지. WIP 커밋 금지 (사용자 지시). 현재 master 작업 트리는 그대로 두고 실행 중 세션 보호
- 주의: worktree는 master HEAD 기준이라 master 미커밋 변경(.claude/agents, hooks, settings.json, repl.go, orch*.go 등)이 포함되지 않음. 대상 파일 줄 번호·내용이 HEAD와 다를 수 있으므로 worktree 생성 직후 1.1·1.3 grep을 재실행해 확인. 미커밋 파일 반영 방법은 U10 (사용자 결정)
- 원본 보존: 수정 전 대상 파일을 `notes/archive/261030/caveman-relay-orig/` 하위에 복사 (기존 `notes/archive/261030/` 파일과 충돌 회피). 원본 삭제 금지
- 구현·훅·튜닝은 worktree에서만. 병합은 sonnet-reviewer 검토 + 사용자 승인

## 4. 테스트 방법
- 4.0 기준선 (변경 전, worktree 생성 직후): `go test ./internal/harness/...` 결과를 `notes/runs/261030-caveman-relay-baseline-gotest.txt`에 저장. 미커밋 테스트가 이미 실패할 수 있음
- 4.1 정적 검사: 파일별 기대 상태 표로 검사 (존댓말 잔존 0건 기준 아님)
  - TUI 출력 대상 (U1): 존댓말 유지
  - 나머지 1.1 파일 + CLAUDE.md:4: "Korean (no honorifics)" 또는 1.1 변경안 존재
  - 1.3 차단 지점 목록 대비 허용 외 경로 목록 확인
- 4.2 훅 단위 테스트: 입력 JSON을 훅에 파이프, `python3 -I` 실행, exit code·decision 확인. enforce와 log 모드 각각 실행
  - Read notes/FACTS.md → deny (log는 would-deny)
  - Read 절대경로 / 상대경로 / `../` 경유 notes 경로 → deny
  - Read internal/harness/repl.go → allow
  - Edit 소스 파일 → allow
  - Read CLAUDE.md → allow (U3)
  - Read `~/.claude/projects/*/memory/x.md` → deny
  - Read `~/.claude/CLAUDE.md` → deny
  - Grep path 생략 (cwd 전체) → notes/ 제외 처리 또는 deny 확인
  - Grep path 지정 `notes/` → deny
  - Glob "notes/**" → deny
  - Bash "cat notes/LESSONS.md", "rg x notes/" → deny
  - Bash 우회: "cat ../claude-harness/notes/X.md" → deny (상대경로·../). 변수 확장·심볼릭 링크는 미탐지 허용, 계획서에 명시
  - Bash "go test ./..." → allow
  - Write notes/runs/x.md → allow
  - WebFetch / WebSearch → 정책 결정값대로 (미확정: 1단계 log에서 would-deny 집계 후 사용자 확인)
  - agent_id 있음·없음 각각 동일 결과
  - log 모드 (`CAVEMAN_GUARD=log`): deny 대신 기록만, exit 0
  - 오류 기본 동작: 잘못된 JSON → allow + stderr 경고. `.claude/context-allow.txt` 없음 → denylist만 적용
- 4.2b 회귀 (item 6, E4): main-thread-guard.py 기존 동작 유지, ledger.py가 LEDGER.md 계속 기록, session-context.py SessionStart 출력에 notes 미포함, style-adapter.py `[inbox]` 미주입
- 4.3 Go 테스트 회귀 판정: 변경 후 `go test ./internal/harness/...` 결과를 4.0 기준선과 비교. 신규 실패 0 = PASS. 기준선부터 실패한 테스트는 회귀 아님 (orch_test, usagepoll_test 등 미커밋 테스트 포함). classifier sha256 테스트 포함
- 4.4 통합 테스트 (2단계): 과제 T1(튜닝)·T2(홀드아웃)를 조건 A(기존)·B(릴레이)로 실행. 측정은 참조 5절 지표
- 4.5 기록: 각 실행 결과를 `notes/runs/261030-result-caveman-relay-impl-<n>.md`에 저장
- 통합 테스트는 worktree에서만 실행. 현재 master 세션 설정 파일 무변경을 `git diff --stat`으로 확인

## 5. 자동 튜닝 루프 (2단계, item 4)
- 5.1 노이즈 기준선 먼저: 루프 시작 전 A 조건 2회, B 조건(초기안) 2회. 지표별 |run1-run2| / 평균 = 노이즈 폭
  - "개선" = 노이즈 폭을 초과한 향상만 인정
  - 노이즈 15% 초과 시 U8 30% 판정 보류, 사용자 보고
- 5.2 평가셋 고정: 튜닝 과제 T1 1개 + 홀드아웃 과제 T2 1개를 루프 시작 전 확정. 지표·채점 기준도 동결. 루프 중 T2 실행 금지 (5.5 (a) 조건에서만 1회)
- 5.3 튜닝 변수: caveman 스타일 지시 문구, 번역 프롬프트 문구 2개만
  - 허용 문서 목록(context-allow.txt) 항목은 튜닝 변수에서 제외. 지표를 올리려 문서를 추가하는 것은 차단 목적 훼손(보상 해킹)
  - 허용 목록 정책, 참조 6절 임계값 변경 금지
- 5.4 1회 루프: 변경 1개 (문구 길이 증가 상한 +20%) → 4.1~4.3 + 번역 검사(참조 9절) PASS 전제 → 4.4 T1 1회 → 지표 기록 → 다음 루프
  - 게이트 실패 시 그 루프는 되돌리고 카운트 제외. 무효 2회 연속이면 중단
- 5.5 종료 조건 (하나라도 충족 시 종료):
  - (a) T1 "유망" 기준 충족 → T2 1회 검증. T2도 충족 시 성공 종료. T2 미충족 시 "과적합" 사유로 종료, 최고안 보고
  - (b) 최대 5회
  - (c) 연속 2회 노이즈 폭 이내 또는 악화
  - (d) 누적 토큰 상한 (U6 기본값: 기준 A 1회 토큰 × 8 = 기준선 2회 + 루프 5회 + 홀드아웃 1회 분량. 금액 상한은 사용자 결정)
  - (e) enforce 모드에서 정상 작업 오탐 deny 1건 이상 → 즉시 중단
- 5.6 채택: 마지막 안이 아니라 지표 최고안. 각 루프 diff·지표를 `notes/runs/261030-result-caveman-relay-impl-<n>.md`에 기록, 종료 시 최고안으로 복원
- 과적합 주의: 1과제 기준이라 일반화 불가 (참조 4절 limit 동일). T2로 일부 보완

## 6. 미확정 사항 및 처리
- U1 TUI 출력 haiku 대상: 사용자 응답 대기. 응답 전 기본 = 1.5 임시안
- U2 초기 컨텍스트 지정 방식: 기본 채택 = `.claude/context-allow.txt` 단일 관리 (1.4)
- U3 CLAUDE.md 자동 로드: 기본 채택 = 차단하지 않음, 문구만 수정, allowlist 고정 포함
- U4 classifier.go 문구: 기본 채택 = 1단계 변경 안 함
- U5 notes 재읽기 금지 강제 수단: 기본 채택 = 훅으로 충분. 지시 제거(2.5) + deny 로그 집계
- U6 튜닝 상한: 횟수 기본 = 최대 5회, 토큰 기본 = 기준 × 8 (5.5-d). 금액 상한은 사용자 응답 대기
- U7 참조 read 계획서 10절 제약(opus-architect만 호출) 개정: 기본 채택 = 1단계 보류. 2단계 착수 시 재검토
- U8 참조 계획서 6절 임계값 30%: 기본 채택 = 유지. 노이즈 15% 초과 시 판정 보류 (5.1)
- U9 FACTS/DECISIONS 존댓말 기록: 사용자 결정 아님. 실행 전 grep 결과 기입 (1.2). 기록 있으면 같은 단계 갱신
- U10 미커밋 변경 반영: 사용자 응답 대기. WIP 커밋·stash 금지 (사용자 지시)이므로 worktree는 master HEAD 기준. 미커밋 파일을 worktree에 반영할 방법(파일 복사 등)을 사용자가 결정 (3절 주의)

사용자 응답 필요 3건: U1, U6 금액, U10. 나머지는 위 기본값 적용.

## 7. 완료 기준
- U1·U6·U10 사용자 응답, 나머지 7건 기본값 기록 (6절)
- 브랜치 caveman-relay-impl 생성 (worktree), 원본 아카이브 존재
- 4.0 기준선 저장, 4.1·4.2·4.2b·4.3 PASS (4.3은 기준선 대비 신규 실패 0)
- 1단계: log 모드 1세션 would-deny 오탐 0 확인 후 enforce 전환
- 4.4 결과 파일 존재, 튜닝 종료 사유 1개 기록, T2 홀드아웃 결과 기록
- 바꾸면 안 되는 것: 참조 계획서 3개, 현재 master 작업 트리의 .claude/settings.json·hooks (병합 전), classifier.go (1단계)

## 8. 병합 반영 확인
- (1) 훅 log-only 1세션 → 차단 전환, 메모리·외부 문서만 차단, 소스 읽기 허용: 2.2, 4.2
- (2) CLAUDE.md 존댓말 문구·notes 재독 지시 제거, 자동 메모리·~/.claude/CLAUDE.md·/resume 스냅샷 차단 지점 추가: 1.1, 1.3, 2.5
- (3) 미확정 10건 중 U1·U6·U10 제외 7건 기본값 채택: 6절
- (4) 노이즈 기준선 선측정, 홀드아웃 분리, 허용 문서 목록 튜닝 변수 제외, 최고 지표 채택: 5.1, 5.2, 5.3, 5.6
- (5) `git worktree add -b caveman-relay-impl <경로> master`, stash·WIP 커밋 금지: 3절
- (6) 훅 케이스 (Bash 우회, 상대경로·../, 경로 없는 Grep, 메모리 경로, Write 허용, log 모드, 오류 기본 동작), 변경 전 Go 기준선 저장 후 회귀 판정: 4.0, 4.2, 4.2b, 4.3
