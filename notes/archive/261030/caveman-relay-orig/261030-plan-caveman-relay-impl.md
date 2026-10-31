# 구현 계획서: caveman 릴레이 (존댓말 제한 해제 + 메모리·외부 문서 차단 + 자동 튜닝)
date: 261030
status: 계획만 작성. 코드·설정 수정 없음. 브랜치 생성 전
refs_read_only: notes/runs/261030-plan-caveman-relay.md, 261030-plan-caveman-relay-read.md, 261030-plan-caveman-relay-flow.md (수정 금지, 읽기만)

## 0. 요청 요지
item_1: (a) TUI 출력용 haiku를 제외한 모든 모델의 존댓말 제한 해제 (b) 모든 모델이 메모리 또는 기타 문서에서 맥락을 가져오는 것 차단 (c) chaude가 지정한 문서만 초기 맥락으로 허용
item_2: 반복 테스트로 자동 튜닝
item_3: 별도 브랜치에 구현 후 테스트
extra: 입력·읽기 결과 caveman화 (참조 3개 계획서 범위)

## 1. 대상 파일 목록 (grep 확인 경로만)

### 1.1 존댓말 규칙 (.claude/agents/*.md)
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
- CLAUDE.md:4 ("Korean, 존댓말." 전역 규칙. 에이전트 파일이 아니나 같은 규칙의 원천)

### 1.2 존댓말 규칙 외 연관 (목록만, 실제 변경 여부 미확정)
- internal/harness/classifier.go:20, :31 — 라우터 프롬프트 task 문구 "no honorifics ... reply: Korean 존댓말". Go 파일이며 "keep this text byte-identical (sha256 test)" 주석 있음 (classifier.go:18). 변경 시 sha256 테스트 갱신 필요
- notes/FACTS.md, notes/DECISIONS.md — 존댓말 규칙 기록 여부 미확인 (grep 결과 목록에 미포함. 실행 전 `grep -n "존댓말" notes/FACTS.md notes/DECISIONS.md` 재확인 필요)

### 1.3 메모리·외부 문서 접근 지점 (.claude/hooks/*.py, internal/)
- .claude/hooks/session-context.py:115 `head("notes/OPEN.md", 20)`, :118 `tail("notes/LEDGER.md", 5)`, :154 `tail("notes/INBOX.md", 50)` — SessionStart 시 notes 문서 자동 주입 (차단 후보)
- .claude/hooks/style-adapter.py:28 `inbox = notes/INBOX.md`, :176 `[inbox]` 태그 주입 (차단 후보)
- .claude/hooks/style-adapter.py:182-187 `notes/pastes/<hash>.md` 경로 안내 (차단 후보)
- .claude/hooks/ledger.py:85 `.claude/agents/<kind>.md` 읽기, :97 `notes/LEDGER.md` 쓰기 (SubagentStop 훅. 읽기 주입 여부 미확인)
- .claude/hooks/main-thread-guard.py:8 WORK_TOOLS = {Read, Grep, Glob, Bash, WebFetch, WebSearch ...}. 메인 스레드에만 작동하며 agent_id 있으면 통과 (main-thread-guard.py:24). 서브에이전트 차단 로직 없음 → 신규 훅 필요
- .claude/hooks/agent-model-pin.py:4 — 모델 pin 관련 (차단 로직 아님. 참고만)
- internal/harness/importclaude.go:156 "Re-read notes/ (FACTS.md, DECISIONS.md) for durable state." — 재읽기 지시 문구 (차단 후보)
- internal/harness/handoff.go:74 withReco — 이전 추천 주입 (메모리성 맥락. 해당 여부 미확정)
- internal/harness/repl.go:252 `--append-system-prompt` (rule 주입. 맥락 문서는 아님)
- internal/harness/core.go:146 `claude -p ... --model` 자식 호출 인자 (초기 맥락 지정 지점 후보)

### 1.4 chaude 초기 컨텍스트 지정 방식
status: 미확정
- grep "initial|--file|@\S+\.md|prepend|seed", "add-dir", "--settings" 결과 chaude 전용 초기 문서 지정 플래그 미발견
- 기존 후보: `--append-system-prompt` (repl.go:252), CLAUDE.md 자동 로드 (Claude Code 기본, session-context.py:18 주석 "launch 시 적용")
- 필요 결정: chaude가 허용할 문서 목록을 어디에 둘지 (예: `.claude/context-allow.txt` 신설 vs settings.json 항목 vs CLI 플래그 `--context`)

### 1.5 TUI 출력용 haiku (존댓말 제외 대상)
status: 미확정
- 어느 에이전트/세션이 TUI 출력인지 식별 불가. internal/harness/tui.go, repl.go 에서 "haiku" 지정 지점 미확인
- 실행 전 `grep -rn "haiku" internal/harness/tui.go internal/harness/repl.go internal/harness/render*.go` 로 확인 필요

## 2. 구현 방식 (안)
- 2.1 존댓말 해제: 1.1 목록 중 TUI haiku 외 파일의 "Korean 존댓말" 문구를 "Korean (no honorifics)"로 교체. 대상 확정 전 변경 금지
- 2.2 메모리·외부 문서 차단: 신규 PreToolUse 훅 (예: .claude/hooks/context-allowlist.py). Read/Grep/Glob/Bash/WebFetch 입력 경로가 허용 목록 밖이면 deny. 서브에이전트 포함 모든 agent_id 대상
- 2.3 초기 맥락: 허용 목록 1개 파일로 관리. chaude가 이 목록에 있는 문서만 초기 맥락으로 주입
- 2.4 session-context.py, style-adapter.py 의 notes 주입은 허용 목록 기준으로 축소 또는 차단
- 2.5 브리프 내 "notes/ 재읽기" 지시는 훅으로 탐지 불가 (텍스트 내용). 금지 문구를 에이전트 파일에 명시하고 haiku-checker로 샘플 검사 (미확정: 강제 수단 없음)
- 2.6 caveman 릴레이: 참조 계획서(2절 형식, 9절 번역 검사) 그대로 사용. 읽기 흐름은 참조 read 계획서 2.6 스키마 적용

## 3. 브랜치
- 이름 제안: caveman-relay-impl
- 생성 명령 (실행 승인 후): `git checkout -b caveman-relay-impl` (main 기준 분기. 현재 master 브랜치에 미커밋 변경 다수 존재 → 분기 전 상태 확인 필요)
- 원본 보존: 수정 전 대상 파일을 notes/archive/261030/ 에 복사 (원본 삭제 금지)

## 4. 테스트 방법
- 4.1 정적 검사: 1.1 목록에서 "존댓말" 잔존 수 (TUI haiku 제외 0건 목표), 1.3 목록의 허용 외 경로 접근 시도 deny 확인
- 4.2 훅 단위 테스트: 허용 문서 Read = allow, 허용 외 notes/ 또는 CLAUDE.md 외 문서 Read = deny, agent_id 있는 호출도 동일 적용. 입력 JSON 샘플 5건 이상
- 4.3 Go 테스트: `go test ./internal/harness/...` (classifier sha256 테스트 포함). 실패 시 원인 기록
- 4.4 통합 테스트: 과제 1개를 조건 A(기존)·B(릴레이)로 실행 (참조 caveman-relay 계획서 7절 설계). 측정은 참조 5절 지표
- 4.5 기록: 각 실행 결과를 notes/runs/261030-result-caveman-relay-impl-<n>.md 에 저장

## 5. 자동 튜닝 루프
- 튜닝 변수: caveman 스타일 지시 문구, 번역 프롬프트 문구, 허용 문서 목록 항목
- 1회 루프: 변경 1개 → 4.1~4.3 실행 → 4.4 1회 → 지표 기록 → 다음 루프
- 종료 조건 (하나라도 충족 시 종료):
  - (a) 4.1·4.2·4.3 모두 PASS 이고 4.4 판정이 참조 6절 "유망" 기준 충족
  - (b) 최대 반복 횟수 도달: 기본 5회 (미확정. 사용자 확정 필요)
  - (c) 연속 2회 개선 없음 (지표 변화 없거나 악화)
  - (d) 누적 비용 상한 도달 (미확정 금액. 사용자 확정 필요)
- 튜닝 중 변경 금지 항목: 허용 목록 정책 자체, 참조 계획서 6절 임계값 (변경 시 사유 기록 필수)
- 과적합 주의: 1과제 기준이라 일반화 불가 (참조 4절 limit 동일)

## 6. 미확정 사항 (사용자 확정 필요)
- U1: TUI 출력 haiku가 어느 세션·에이전트인지 (1.5)
- U2: chaude 초기 컨텍스트 지정 방식 (CLI 플래그 / 파일 / settings) (1.4)
- U3: CLAUDE.md 자동 로드를 차단 대상으로 볼지 (Claude Code 기본 동작이며 CLAUDE.md 는 사용자 지정 규칙 문서)
- U4: classifier.go 라우터 문구 변경 허용 여부 (sha256 테스트 갱신 포함)
- U5: 브리프 내 "notes/ 재읽기" 지시 금지의 강제 수단 (훅 불가, 샘플 검사만 가능)
- U6: 자동 튜닝 최대 반복 횟수, 비용 상한
- U7: 참조 read 계획서 10절 제약 (opus-architect만 호출) 개정 여부 — 읽기 흐름 적용 전 필요
- U8: 참조 계획서 6절 임계값 30% 확정 여부
- U9: notes/FACTS.md, notes/DECISIONS.md 의 존댓말 규칙 기록 존재 여부 (재확인 필요)
- U10: 현재 master 에 미커밋 변경 다수 (git status 기준). 브랜치 분기 시 포함 여부

## 7. 완료 기준
- 본 계획서 확정 (U1~U10 응답)
- 브랜치 caveman-relay-impl 생성, 원본 아카이브 존재
- 4.1~4.3 PASS 기록, 4.4 결과 파일 존재
- 튜닝 종료 사유 1개 기록
