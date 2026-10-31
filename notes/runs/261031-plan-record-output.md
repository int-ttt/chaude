# plan: record-output
format: plan/caveman-structure.md 3.1~3.6 (정본 docs/claude-lean/CAVEMAN_FORMAT.md)
rev: 261031 caveman 재작성. 구판(61행, 마크다운 산문 7절)의 내용과 리뷰 반영분을 통합, 구판 내부 모순 2건 정리 (S1 recordIndexAppend 제거, 분석 턴 저장 기준 통일)

## R-decide
status: done
reason: -
decision: 프로젝트 루트 record/에 TUI 출력을 턴 단위로 append 저장하고, /resume은 record 파일을 출력창에 표시만 하며, 기존 Claude 세션 재개는 /resume claude로 옮긴다
why: E1 사용자 목표 8항; E2 리뷰 결정(index.jsonl 삭제, 번호는 파일 헤더, 분석 턴도 저장, 로드 블록 재기록 금지, 화면 표시 전용); E3~E6 삽입 지점
rejected: record/index.jsonl 색인: 리뷰에서 삭제, 시작 시 최신 record 파일 마지막 rec 번호 +1로 대체 / 분석·측정 턴 저장 제외: 리뷰에서 출력창 사본은 저장으로 결정, 파일 생성 금지는 프롬프트 규칙으로 분리 / 접두어 기반 분석 판정: 오판 위험, 첫 토큰 정확 일치 + /norec로 대체 / notes/ 기존 Go 경로·hooks 동시 이전: 범위 과대, Q1 기본 제외 / /resume 로드 블록을 모델 컨텍스트에 주입: 리뷰에서 화면 표시 전용으로 결정
risks: R1 저장 중단 시 파일 불완전 -> 한 턴 = O_APPEND 한 번, doneMsg 기준 / R2 resumeshift_test.go importedMsg 경로 영향 -> 기존 테스트 유지, D7 문구 변경분만 수정 / R3 비밀값이 record/에 평문으로 남음 -> 차단 수단 없음, CLAUDE.md에 명시 / R4 rotation·자식 교체 때 이중 기록 -> 부모 TUI 한 곳에서만 기록, 쓰기 flock / R5 일괄 로드 블록 과다 -> 최근 500블록 상한 / R6 [cat-switch: 신호 도착 지점이 없을 수 있음 -> S0 결과에 따라 /topic <이름> 명령 또는 첫 사용자 입력 앞 20자 슬러그 / R7 CLAUDE.md는 현재 미커밋 수정 상태 -> S6은 줄 추가만
open_q: Q1 notes/ 중단을 hooks와 Go 기본 경로까지 넓힐지 (기본: 이번 변경 제외) / Q2 record/를 .gitignore에 추가할지 (권고: 추가, 사용자 최종 확인 필요) / Q3 첫 토큰 {분석, 측정} 일치 턴을 record 저장에서 뺄지 (기본: 저장 유지, 분석 결과 파일 생성 금지 신호로만 사용)
plan_path: /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261031-plan-record-output.md
summary_ko: TUI 출력은 record/ 폴더에 날짜·주제별 파일로 턴마다 덧붙여 저장하고, /resume으로 화면에 다시 불러옵니다. 기존 Claude 세션 재개는 /resume claude로 옮깁니다. 확인이 필요한 것은 notes/ 중단 범위, record/ 커밋 여부, 분석 턴 저장 여부 세 가지입니다.

## evidence (R-read)
status: partial
reason: E3~E6 코드 행 번호는 구판 인용이며 이번 작업에서 소스로 재확인하지 않음 (S0에서 확인)
claim: /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261031-plan-record-output.md:3-10 (구판) | 목표: notes/ 사용 중단, 새 저장소 record/, TUI 출력창 내용 저장, /resume으로 출력창 로드, 날짜·주제 바뀌면 새 파일, 답변 순번 인덱싱, 분할 파일 순서대로 일괄 표시, 분석·측정 작업은 파일을 만들지 않고 대화창 출력만 [확실] (E1)
claim: /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261031-plan-record-output.md:46-61 (구판) | 리뷰 반영: 분석 턴 저장, S0 신호 확인, turnStartIdx 저장 단위, 로드 블록 재기록 금지, rec 주석·헤더 형식, index.jsonl 삭제, 분할 빈칸, 시계·디렉터리 주입, /resume claude [인자], 저장 위치 git-common-dir 상위 또는 HARNESS_RECORD_DIR + flock, 화면 표시 전용·최근 N블록 상한, 비밀값 위험, Q1~Q3 [확실] (E2)
claim: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/tui.go:435-437 | doneMsg 처리 지점 [추정] (E3)
claim: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/tui.go:917-926 | /resume 분기 [추정] (E4)
claim: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/repl.go:1054-1056 | plain loop /resume 처리 [추정] (E5)
claim: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/resumeshift_test.go:- | importedMsg 경로 테스트 [추정] (E6)
claim: /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261031-plan-record-output.md:59 (구판) | Q1 이전 목록: session-context.py:114-154, ledger.py, style-adapter.py:7,187, importclaude.go:156, drain.go:13, switchchild.go:247, metrics·extract_transcript 기본 경로 [확실] (E7)
claim: -:- | [cat-switch:·[cat: 신호가 TUI에 도착하는 지점 미확인 [불확실] (E8)
searched: 구판 plan 1-61행; plan/caveman-structure.md 1-279행 발췌; docs/claude-lean/CAVEMAN_FORMAT.md 전체; notes/LESSONS.md 전체; notes/DECISIONS.md grep caveman|plan|record|output|3.6
not_found: notes/LESSONS.md·notes/DECISIONS.md에 record-output 관련 항목 없음
not_checked: E3~E6 실제 소스 행; 리뷰 파일 notes/runs/261031-plan-record-output-review.md 원문; E8 신호 도착 지점
truncated: no ; omitted: -

## defaults (D1-D9, 구현 시 재결정 금지)
D1_store: /Users/leekwangseok/PycharmProjects/claude-harness/record/ (없으면 생성). worktree에서는 git-common-dir 상위 폴더 고정. 환경변수 HARNESS_RECORD_DIR로 재정의. 쓰기는 flock
D2_name: record/YYMMDD-<주제슬러그>-<NNN>.md (NNN = 같은 날·같은 주제 안의 분할 번호, 001부터)
D3_split: (a) 턴 종료 로컬 날짜가 바뀜 (b) 카테고리 전환 신호 [cat-switch: X] 수신, A-B-A 전환 시 새 NNN 파일 (c) 파일당 200KB 또는 답변 100개 상한 도달 시 NNN 증가
D3_slug: 한글 유지, 공백과 / : * ? -> -, 최대 30자, 빈 값은 misc
D4_seq: 답변마다 전역 순번 #0001부터 증가. 시작 시 최신 record 파일의 마지막 rec 번호 +1. record/index.jsonl 만들지 않음
D4_block: 블록마다 `<!-- rec #0042 asst 2026-10-31T14:05 cat=<주제> -->` 주석 다음 줄에 `## #0042 · 14:05 · <주제>` 헤더 (왕복 파싱 가능해야 함)
D5_analysis: 분석·측정 턴도 record에 저장 (출력창 사본). 분석 결과 파일 생성 금지는 에이전트 프롬프트·CLAUDE.md 규칙으로 따로 둠
D5_unit: 턴 시작 시 turnStartIdx=len(blocks). doneMsg에서 blocks[turnStartIdx:]의 user·asst·note 본문 저장, tool·result는 한 줄 요약. 한 턴 = O_APPEND 한 번
D6_resume: 인자 없음 -> 최근 record 파일 목록 최대 30개 picker. 인자 <파일명|날짜|주제> -> 해당 분할 파일을 NNN 순으로 읽어 일괄 표시. 화면 표시만, 모델 컨텍스트 주입 없음. 상수 recordLoadMaxBlocks = 500 (최근 블록만)
D6_loaded: 로드된 블록 재기록 금지 (block.loaded=true, 로드 후 turnStartIdx=len(blocks))
D7_claude: 기존 /resume(Claude 세션 JSONL)은 /resume claude [인자]로 이동. 도움말 문구 갱신. /import 유지
D8_notes: 새 기능은 notes/ 미사용. 기존 Go 기본 경로(metrics, distill)·hooks(LEDGER, INBOX, pastes)는 변경 없음 (Q1)
D9_norec: /norec 수동 토글 = 다음 토글까지 record 저장 끔

## steps (B형식, S0부터 순서대로)
### S0 -> haiku-scout
task: [cat-switch:·[cat: 신호 도착 지점과 E3~E6 행 번호 확인
paths: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/tui.go, /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/repl.go, /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/resume.go, /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/resumeshift_test.go
ctx: E3: tui.go:435-437 / doneMsg [추정] ; E4: tui.go:917-926 / /resume 분기 [추정] ; E5: repl.go:1054-1056 / plain loop [추정] ; E8: grep 패턴 '\[cat-switch:|\[cat:' (internal/ 전체)
limits: 읽기만, 수정 금지
return: R-read, 3000자
lang: caveman

### S1 -> haiku-coder
task: record 쓰기 모듈 신규 작성
paths: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/record.go (신규), /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/record_test.go (신규)
ctx: D1~D5, D9 그대로 ; 함수 recordAppendTurn, recordSplitIfNeeded (recordIndexAppend 만들지 않음, D4) ; 시계·디렉터리 주입 가능 ; 테스트: 자정 경계, A-B-A, 한글 슬러그, 200KB·100개 분할, 왕복 파싱, 재시작 후 번호 이어짐 ; check: go test ./internal/harness/ -run Record
limits: 기존 파일 수정 금지, notes/ 경로 사용 금지
return: R-write, 15줄
lang: caveman

### S2 -> haiku-coder
task: tui.go 턴 종료 시 record 저장 연결
paths: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/tui.go:435-437 (S0 확인 행 우선)
ctx: D5_unit ; doneMsg 처리 뒤 recordAppendTurn(blocks[turnStartIdx:]) ; 카테고리 신호는 S0 지점에서 전달, 없으면 /topic <이름> 명령 또는 첫 사용자 입력 앞 20자 슬러그 (R6) ; 기록은 부모 TUI에서만 (R4) ; /norec 토글 (D9)
limits: record.go 시그니처 변경 금지, resumeshift_test.go 기존 케이스 삭제 금지
return: R-write, 15줄
lang: caveman

### S3 -> haiku-coder
task: record 분할 파일 로더 loadRecord 추가
paths: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/resume.go, /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/resume_test.go
ctx: loadRecord(dir, selector) — selector = 파일명|날짜|주제 ; 분할 파일 NNN 순으로 읽어 blocks 목록 반환 ; D4_block 주석으로 블록 복원 ; block.loaded=true ; recordLoadMaxBlocks = 500 최근 블록만 (D6_resume)
limits: 기존 Claude JSONL 로드 동작 변경 금지
return: R-write, 15줄
lang: caveman

### S4 -> haiku-coder
task: tui.go /resume 분기를 record 로드로 교체, 기존 동작은 /resume claude로 이동
paths: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/tui.go:917-926 (S0 확인 행 우선), /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/resumeshift_test.go
ctx: D6_resume, D6_loaded, D7_claude ; 인자 없음 -> record 목록 picker 최대 30개 -> 선택 시 loadRecord 결과를 importedMsg와 같은 방식으로 blocks에 넣고 turnStartIdx=len(blocks) ; /resume claude [인자] -> 기존 Claude 세션 목록 ; resumeshift_test는 D7 문구 변경분만 수정 ; 로드 블록 재기록 금지 테스트 추가
limits: /import 변경 금지, 로드 블록을 모델 컨텍스트에 주입 금지
return: R-write, 15줄
lang: caveman

### S5 -> haiku-coder
task: repl.go plain loop /resume에 S4와 같은 동작 적용
paths: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/repl.go:1054-1056 (S0 확인 행 우선)
ctx: S4와 같은 동작 ; picker가 없으면 번호 목록 출력 후 번호 입력 ; /resume claude 동일
limits: tui.go 수정 금지
return: R-write, 10줄
lang: caveman

### S6 -> haiku-batch-worker
task: CLAUDE.md에 record 규칙 2줄 추가
paths: /Users/leekwangseok/PycharmProjects/claude-harness/CLAUDE.md
ctx: 추가 원문 1: "새 기록은 record/, notes/는 읽기 전용 이전 대상" (E7) ; 추가 원문 2: "record/에는 출력창 내용이 평문으로 남음, 비밀값 주의" (R3)
limits: 기존 줄 수정·삭제 금지 (R7), 다른 파일 금지
return: R-write, 5줄
lang: caveman

### S7 -> haiku-runner
task: 빌드·정적검사·테스트 실행
paths: /Users/leekwangseok/PycharmProjects/claude-harness
ctx: go build ./... ; go vet ./... ; go test ./internal/harness/...
limits: 소스 수정 금지
return: R-write, 10줄 (check: <명령> -> PASS|FAIL)
lang: caveman

### S8 -> haiku-checker
task: accept 항목 대조 확인
paths: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/, /Users/leekwangseok/PycharmProjects/claude-harness/CLAUDE.md
ctx: 아래 accept 6항, keep 4항
limits: 읽기·테스트 실행만, 수정 금지
return: R-write, 10줄
lang: caveman

## edge
edge: 자정 경계 -> 턴 종료 시각 기준 날짜
edge: A-B-A 전환 -> 세 번째 A는 새 NNN 파일
edge: 재시작 -> 최신 record 파일 마지막 rec 번호 +1
edge: 로드된 블록 -> 재기록 금지
edge: 빈 슬러그 -> misc
edge: rotation·자식 교체 -> 부모 TUI만 기록
edge: record/ 없음 -> 생성

## accept
accept: go build ./..., go vet ./..., go test ./internal/harness/... -> PASS
accept: record_test 자정 경계, A-B-A, 한글 슬러그, 200KB·100개 분할, 왕복 파싱, 재시작 후 번호 이어짐 -> PASS
accept: 로드 블록 재기록 금지 테스트 -> PASS
accept: /resume claude -> 기존 Claude 세션 목록 표시
accept: record/index.jsonl 생성 안 됨
accept: CLAUDE.md에 S6 원문 2줄 존재

## keep (바꾸지 않는 것)
keep: /import 동작
keep: notes/ 기존 Go 기본 경로(metrics, distill)·hooks(LEDGER, INBOX, pastes) (Q1)
keep: resumeshift_test.go 기존 케이스 (D7 문구 변경분 제외)
keep: CLAUDE.md 기존 줄

## decisions 261101 (판단 위임 b·c·d·e, 버전 v0.9.0)
user: "v0.9.0이 예상 버전이었음; b, c, d, e 항목은 에이전트가 알아서 결정"
a_version: cmd/chaude/main.go:16 "0.9.0-r1" -> "0.9.0" | evidence: HEAD 4540c65 = "0.9.1", worktree "0.9.0-r1", only version literal (git grep); CHANGELOG.md:3 "## 0.9.0" | check: chaude --version -> "chaude 0.9.0" | untouched: .claude/claude-lean.version 0.5.3 (installed marker, separate)
a_keep: recTopic, /resume general-mode branch kept | evidence: go build/vet exit 0, go test ./internal/harness/... 183 PASS 0 FAIL
b_distill: keep extract_transcript default notes/distill and haiku-extractor paths | why: distill = knowledge-candidate pipeline feeding notes/, not output copy; 26 refs/11 files incl .claude/skills/distill/SKILL.md (7) and distributed docs/claude-lean README/FRESH_START/CHANGELOG; notes/distill absent now -> nothing to migrate; record-scope Q1 excluded it | evidence: internal/scripts/extract_transcript.go:34; .claude/agents/haiku-extractor.md:3,13 | risk: notes/distill not in .gitignore -> leftover chunks (raw transcript) could be committed
c_ledger_inbox: keep notes/LEDGER.md in place as read fallback; no move, no copy; INBOX nothing to do | why: notes/LEDGER.md git-tracked (83382 B), no-deletion rule; record/LEDGER.md exists so pick() reads record first; copy = 83KB duplicate in gitignored dir | evidence: .claude/hooks/session-context.py:125,127 pick("record/LEDGER.md","notes/LEDGER.md"), pick("record/INBOX.md","notes/INBOX.md"); git ls-files -> notes/LEDGER.md only; notes/INBOX.md, record/INBOX.md absent; ledger.py:98 writes record/LEDGER.md
d_pyright: fixed, 1 line | cause: not unbound (upd bound :17 module scope); :38 dict literal inferred dict[str, dict[str, str|Unknown]] -> :40 str assign = argument type error (strict only; basic 0) | change: session-context.py:38 `msg = {` -> `msg: dict[str, object] = {` | check: pyright strict 38-41 0 errors (before :38 partially unknown, :40 argument type); hook tests Ran 33 OK
e_tui_measure: method = one real turn via expect, HARNESS_RECORD_DIR=/tmp/chaude-live/rec | record: verified, /tmp/chaude-live/rec/261101-인사-001.md:11 "[측정] duration_ms=3114 cost_turn_usd=0.00413 ... result_output_tokens=260", reply :9 | pane: 미확인, expect log (alt-screen, 4818 B) has neither reply text nor 측정 -> capture limit, not proof of absence | next: 실제 터미널에서 눈으로 확인 or tmux capture-pane (tmux 미설치) | side: metrics file 2026-10-31.jsonl (UTC date) vs record 261101 (local)
known_gap: budget-stop path may skip 측정 line (repl.go ~898-900), unchanged
reviewer: sonnet-reviewer not run from opus-architect (LESSONS.md:5, agent-model-pin haiku-* only) -> main thread must dispatch
