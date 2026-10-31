# plan: record scope 확장 (caveman-structure.md 형식)
status: plan v2 (261031, review merged)
supersedes: notes/runs/261031-plan-record-output.md Q1/Q2/Q3 기본값 (사용자 정정)
user_words: "caveman-structure.md대로 구현 재요청(사용자 정정, 기존 구현 불일치). 1 범위 넓힘; 2 항목 추가; 3 분석 수행, 측정 결과 TUI 출력 포함."
fmt: briefs = 3.1 B형식; returns = 3.2 R-read / 3.3 R-write / 3.4 R-decide; 3.5 빈 필드 "-"; 3.6 status 헤더 (spec plan/caveman-structure.md:53-121)

## 3.4 R-decide
status: done
reason: -
decision: hooks·Go 기본 자동 기록을 notes/에서 record/로 옮기고(읽기는 notes/ 레거시 fallback), record/ ignore 확인, 분석·측정 턴도 record 저장, 턴별 측정 줄을 출력창에 찍고 record에도 저장
why: E2 record_turn.go:17 분석/측정 skip = 사용자 3번과 충돌; E3 hooks 4곳·Go 3곳 notes/ 기본 쓰기; E4 .gitignore:14 record/ 이미 있음
rejected: Go만 이전(hooks 유지): 사용자 1번이 hooks 명시 / notes 파일 이동·삭제: 원본 삭제 금지 제약 / statusLine에 측정 상시 표시: 턴 사본이 record에 안 남음
risks: 기존 notes/metrics 데이터는 기본 요약에서 빠짐(--dir notes/metrics로 조회) / worktree에서 hooks(CLAUDE_PROJECT_DIR)와 Go(recordDir=main root) 경로가 다를 수 있음
open_q: Q1 extract_transcript 기본 notes/distill 이전 여부 (haiku-extractor 정의가 notes/distill/chunks 참조, 이번 제외) / Q2 기존 notes/LEDGER.md·INBOX.md 내용을 record/로 옮길지 (이번 제외, 읽기 fallback만)
plan_path: notes/runs/261031-plan-record-scope.md

## evidence
E1: plan/caveman-structure.md:53-121 / 3.1-3.6 형식 정의
E2: internal/harness/record_turn.go:10-18 / recordSkip: 첫 토큰 분석|측정 -> true
E3: .claude/hooks/ledger.py:96-105 notes/LEDGER.md; style-adapter.py:26-28,127-134,177,182-187 notes/INBOX.md·pastes; caveman-format.py:110-118 notes/metrics; session-context.py:114-118,154 읽기
E4: .gitignore:14 / "record/"
E5: internal/harness/repl.go:1103 metrics-dir 기본 notes/metrics; metricscmd.go:16 dir 기본 notes/metrics; cmd/chaude/main.go:20 usage "(notes/metrics)"
E6: internal/harness/repl.go:942-957 fields 구성 + r.metrics.Log(fields) (stage=turn)
E7: internal/harness/record_turn.go:92-101 recordSave = prompt + r.lastReply 저장
E8: internal/harness/tui.go:1176-1180 runTUI: r.handle(line); r.recordSave(line); p.Send(doneMsg{})
E9: internal/harness/record.go:59-79 recordDir: HARNESS_RECORD_DIR > <git main root>/record > cwd/record

## D (결정)
D1_dir_hooks: rec = os.environ.get("HARNESS_RECORD_DIR") or os.path.join(root, "record"); root 계산은 각 hook 기존 그대로
D2_map: notes/LEDGER.md -> record/LEDGER.md; notes/INBOX.md -> record/INBOX.md; notes/pastes/<h>.md -> record/pastes/<h>.md; notes/metrics/ -> record/metrics/
D3_read_fallback: session-context.py LEDGER·INBOX 읽기 = record/ 파일 있으면 그것, 없으면 notes/ 파일. notes/OPEN.md 는 그대로 (정리 문서, 자동 기록 아님)
D4_go_metrics: metrics-dir·metricscmd --dir 기본값 "auto" -> filepath.Join(recordDir(cwd), "metrics"); 빈 문자열 = 끔 (기존 의미 유지)
D5_skip: recordSkip = 빈 프롬프트 또는 "/" 시작만 true. 분석·측정 제거
D6_measure: handle()에서 fields로 측정 줄 생성 -> r.lastMeasure; 출력은 r.p 로 reply 뒤에 1줄; recordSave 는 text 뒤에 "\n\n"+r.lastMeasure 덧붙임
D7_measure_fmt: "[측정] " + fields 키 정렬 후 "k=v" 공백 연결. 제외 키: stage, session, flags, ts, 그리고 문자열 값 길이>40. float 은 %.4g
D8_notes_keep: notes/runs/ (drain.go:13, switchchild.go:249), importclaude.go:156 문구, notes/OPEN.md 읽기 = 변경 없음 (에이전트 정리 문서 경로, 자동 기록 아님)

## steps
S0 haiku-scout (3.1 brief, return 3.2 R-read)
task: 대상 행 재확인
paths: internal/harness/repl.go:220-230,900-965,1095-1110,1185-1195; internal/harness/record.go:55-80; internal/harness/metricscmd.go:10-25; internal/harness/tui.go:1170-1182
return: R-read 3000자: r.p 가 TUI 모드에서 어디로 가는지, recordDir 시그니처, a.Cwd 존재, fields 키 목록

S1 haiku-coder (return 3.3 R-write) Go
files: internal/harness/record_turn.go, repl.go, metricscmd.go, cmd/chaude/main.go, record_turn_test.go(신규)
- record_turn.go:10-18 recordSkip 본문 -> `return len(f) == 0 || strings.HasPrefix(f[0], "/")`; 주석 "slash commands and empty prompts" 로 수정
- record_turn.go recordSave: text 만든 뒤 `if r.lastMeasure != "" { text += "\n\n" + r.lastMeasure }`
- repl.go Repl struct: 필드 `lastMeasure string` 추가
- repl.go handle 시작부: `r.lastMeasure = ""`; :957 Log 직후 `r.lastMeasure = measureLine(fields); if r.lastMeasure != "" { r.p(r.lastMeasure, "\n") }`
- record_turn.go 에 `func measureLine(fields map[string]any) string` (D7). 키 0개면 ""
- repl.go:1103 기본값 "auto", 설명 "per-turn JSONL metrics dir (auto=<record>/metrics, empty=off)"; flag parse 후 `if a.MetricsDir == "auto" { a.MetricsDir = filepath.Join(recordDir(a.Cwd), "metrics") }` (a.Cwd 가 아직 비면 os.Getwd 결과 사용)
- metricscmd.go:16 기본 "auto", parse 후 auto -> filepath.Join(recordDir(cwd), "metrics"), cwd=os.Getwd()
- cmd/chaude/main.go:20 "(notes/metrics)" -> "(record/metrics)" 만 수정
- record_turn_test.go: TestRecordSkip ("분석 a"->false, "측정"->false, "/resume"->true, ""->true, "hello"->false); TestMeasureLine (stage/session 제외, 정렬, 빈 map -> "")
check: go build ./... && go vet ./internal/harness/ && go test ./internal/harness/ -run 'RecordSkip|MeasureLine' -count=1

S2 haiku-coder (return 3.3) hooks
- ledger.py:97 path = os.path.join(rec, "LEDGER.md") (D1)
- style-adapter.py:27-28 notes 변수 -> rec 디렉터리(D1), inbox = os.path.join(rec, "INBOX.md"); :127 makedirs(rec); :182 pastes 경로 rec 기준; :187 사용자 문구 "notes/pastes/" -> "record/pastes/"
- caveman-format.py:115 d = os.path.join(os.environ.get("HARNESS_RECORD_DIR") or os.path.join(root, "record"), "metrics")
- session-context.py: helper `def pick(rel_new, rel_old)` -> 존재하는 쪽 rel; :118 tail(pick("record/LEDGER.md","notes/LEDGER.md"),5); :154 tail(pick("record/INBOX.md","notes/INBOX.md"),50); HARNESS_RECORD_DIR 설정 시 그 절대경로 우선; :114 문구 "snapshot of notes/" -> "snapshot of notes/ + record/"
- tests/hooks/test_caveman_format.py:4 docstring notes/metrics -> record/metrics; :55 glob 경로 "record","metrics"; env 에 HARNESS_RECORD_DIR 제거(상속 방지) 
check: python3 -m unittest discover -s tests/hooks -v

S3 haiku-batch-worker (3.3): .gitignore 확인만 (record/ 14행 존재 -> 변경 없음, 없으면 notes/pastes/ 다음 줄에 "record/" 추가)

S4 haiku-runner (3.3): go build ./... ; go vet ./... ; go test ./... -count=1 ; python3 -m unittest discover -s tests/hooks ; 스모크: 임시 CLAUDE_PROJECT_DIR 로 ledger.py·caveman-format.py 실행 후 <tmp>/record/ 생성·<tmp>/notes/ 미생성 확인
return: 통과/실패 개수, 실패 원문 20줄

S5 haiku-checker 파일별 diff 대조 (accept), S6 sonnet-reviewer 전체 diff 리뷰

## edge
- r.lastMeasure 는 handle 마다 초기화 (이전 턴 값 누출 금지)
- 슬래시 명령 턴은 fields 없음 -> 측정 줄 없음
- HARNESS_RECORD_DIR 상대경로 = 그대로 사용
- metricscmd 기존 notes/metrics 데이터: --dir notes/metrics 로 조회 가능

## accept
- grep '"notes"' .claude/hooks/{ledger,style-adapter,caveman-format}.py = 0 (session-context.py 는 fallback·OPEN.md 만)
- grep 'Join("notes", "metrics")' internal/ = 0
- recordSkip("분석 x") == false 테스트 통과
- 턴 종료 시 출력창에 "[측정] " 1줄, record 파일 해당 답변에 같은 줄
- .gitignore 에 record/ 존재
- go test ./..., hooks unittest 통과

## keep (변경 금지)
- notes/ 기존 파일 내용 (이동·삭제 없음), notes/runs/ 규칙, drain.go, switchchild.go, importclaude.go
- extract_transcript.go 기본 notes/distill (Q1)
- CLAUDE.md, .claude/settings*.json, .claude/agents/*
- record.go recordDir 동작, record 파일 형식

## review merged (v2) - 아래가 위 D/S 를 덮어씀
D7_measure_fmt_v2: measureLine(fields) = "[측정] " + 순서 고정 [duration_ms, cost_turn_usd, cost_usd, ctx, rotations, delegations] 중 존재+숫자인 키, 그다음 "result_" 접두 키 중 숫자 값만 사전순. 값 형식: int/int64 -> %d, float64 -> %.4g. "k=v" 공백 연결. 해당 키 0개 -> ""
S1_add: recordSave 끝(append 후, skip return 경로 포함)에서 r.lastMeasure = ""
S1_auto_v2: Main: `if a.MetricsDir == "auto" { a.MetricsDir = filepath.Join(recordDir(a.Cwd), "metrics") }` 를 ParseInterspersed(repl.go:1122) 직후에; metricscmd: recordDir("")
S1_test_v2: TestMeasureLine: fields{"stage":"turn","start_ms":int64(1),"duration_ms":int64(1200),"cost_usd":0.01234,"result_model_usage":map[string]any{},"result_input_tokens":10} -> "[측정] duration_ms=1200 cost_usd=0.01234 result_input_tokens=10"; 빈 map -> ""
S2_test_v2: tests/hooks/test_caveman_format.py env 구성 직후 env.pop("HARNESS_RECORD_DIR", None)
S_tui: tui.go 수정 없음 (r.p -> tuiWriter -> noteMsg, tui.go:53-59,1152-1153)

resolved_261101: open_q Q1(distill), Q2(LEDGER·INBOX) -> 261031-plan-record-output.md "decisions 261101" (b_distill keep, c_ledger_inbox keep fallback)
