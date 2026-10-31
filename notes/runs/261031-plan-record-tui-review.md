# review: 261031-plan-record-tui.md
verdict: keep structure; fix 14 items below before haiku-coder starts. plan conflicts with existing record.go in 3 places (I1-I3) and targets a renamed file (I4).
evidence: E1 plan lines 1-107; E2 internal/harness/record.go:1-231 (untracked, no non-test caller); E3 record_test.go 6 tests; E4 notes/DECISIONS.md:12,15,28,29; E5 notes/LESSONS.md:3; E6 notes/runs/261031-plan-record-output-review.md

## A. align plan with code already written (record.go wins; tests exist)
I1 file name (plan 결정1 line 14): replace `record/YYYY-MM-DD-<topic-slug>[-partN].md` with `record/YYMMDD-<slug>-NNN.md` (record.go:92, recordFileRe :35). NNN = max NNN for date+slug +1; new NNN on: date change, topic re-entry (A-B-A, test TestRecordTopicABA), cap. drop "partN" wording everywhere (S1.2, S3.2, S4.2).
I2 header (결정2 line 20): replace `#순번 · 시각` with the two-line form in record.go:178-179: `<!-- rec #0042 asst 2026-10-31T14:05 cat=<topic> -->` + `## #0042 · 14:05 · <topic>`. number = per turn (user+asst+note+tool/result summaries of one turn share it), global across dates/topics, 4 digits.
I3 open questions 1,2 (lines 105-106) already decided in code: cap = 200KB or 100 asst per file (record.go:29-30); slug = trim, space and /:*? -> '-', max 30 runes, Korean kept, empty -> "misc" (record.go:214-231). move to 결정 3; delete from 열린 질문. add to slug: also replace `<>|"\` and control chars (Windows-unsafe), and `-->` must not survive in cat (see I8).

## B. wrong target / missing scope
I4 S6 targets `CLAUDE.md`, but DECISIONS.md:28 renamed it to `main.md` (injected by SessionStart hook). change S6 and 결정6 to `main.md`. grep first: `grep -rn "CLAUDE.md\|main.md" /Users/leekwangseok/PycharmProjects/claude-harness/.claude /Users/leekwangseok/PycharmProjects/claude-harness/internal`.
I5 "notes 사용 중단" has no file list. add step S6b: `grep -rln "notes/" .claude/agents .claude/hooks main.md internal` -> list every writer (agent .md rules like `notes/runs/<YYMMDD>-plan-...`, hooks session-context.py:114-154 reader, history-export.py). decide per file: writer -> stop writing / point to record/ or chat output; reader -> keep read-only (old notes stay, no delete, no move). must NOT delete notes/.
I6 topic source (S2.2) only reads `[cat-switch: X]`. DECISIONS.md:29: main session appends `[cat: X]` to every reply and `[cat-switch: X]` on change. rule: topic = last `[cat-switch: X]` else last `[cat: X]` in the reply text, else previous topic, else "misc". reuse the existing cat parser used by rotation (grep `cat-switch` in internal/harness/*.go); do not write a second regex. keep the tag in saved text (it is TUI output).
I7 record dir (결정1 "프로젝트 루트") undefined under worktrees (stacked worktrees, commit 56711b6). rule: env HARNESS_RECORD_DIR if set (tests use t.TempDir()), else `<dir of git rev-parse --git-common-dir>/../record` i.e. main checkout root, else cwd/record. one function `recordDir() string`, resolved once at Repl start.

## C. code gaps to add to S1 (record.go is not done)
I8 parser injection: a body line starting `<!-- rec #` splits an entry; `-->` in topic breaks recordHeaderRe. render: prefix any body line matching `^<!-- rec #` with `\`; parse: strip that one `\`. cat: replace `-->` with `->`. add test TestRecordEscapeHeaderInBody.
I9 Windows build: DECISIONS.md:5 ships `bin/harness[.exe]`; syscall.Flock (record.go:61) does not compile on windows. move lock to record_lock_unix.go (`//go:build !windows`, Flock) and record_lock_windows.go (`//go:build windows`, LockFileEx via golang.org/x/sys/windows if already in go.mod, else no-op + comment). signature: `func lockRecordDir(dir string) (unlock func(), err error)`.
I10 nextRecordNumber trusts newest mtime (record.go:128); a touched/edited old file returns a lower number -> duplicate numbers. use max over the last entry of every record file (read only last 8KB of each). keep TestRecordNumberContinuesAfterRestart, add TestRecordNumberIgnoresMtime.
I11 one turn > 200KB still lands in one file (fine) — write this in 결정3 as accepted, so nobody adds splitting inside a turn.

## D. /resume and load (S3-S5)
I12 replace the global "로드 중 플래그" (S3.3, S4.4) — a turn can end asynchronously while /resume renders and get dropped or the flag stays up on error. instead mark blocks shown from record (`fromRecord bool` on the TUI block) and have the recorder skip those blocks; no global flag. /resume output itself (list, notes) is never recorded.
I13 command name: DECISIONS.md:15 hides /resume (Claude native session resume); LESSONS.md:3 had an older import-snapshot /resume. state: harness intercepts `/resume` before passthrough; `/resume` = topic list; `/resume <slug|번호>` = show; native resume moves to `/resume claude [args]`. display only, not injected into model context (open_q 1). do not touch rotate pendingSnapshot (LESSONS.md:3).
I14 "분리 파일 일괄 표시": group by slug across all dates and NNN; order by rec number (global, monotonic) not by file name. list row: `<번호>. <slug> · <first date>~<last date> · 파일 <n>개 · 답변 <n>개`. cap display at last 200 blocks with a note line `앞 <n>개 생략` (avoid freezing TUI). functions: `listRecordTopics(dir string) ([]recordTopic, error)` with recordTopic{Slug string; Files []string; First, Last string; Answers, LastNum int}; `loadRecordTopic(dir, slug string) ([]recordEntry, error)`.
S5 merge: single call site `func (r *Repl) recordTurn(blocks []recordBlock, reply string)` used by TUI turn-end and plain REPL turn-end; S5 becomes "plain REPL calls the same two functions" + test via HARNESS_PLAIN=1 piped stdin (DECISIONS.md:12).
S2.3 detail: record error -> one note block `기록 실패: <err>` shown once per session, never recorded.

## E. 결정5 (분석·측정 no file) needs an owner
- it is an agent-behavior rule, not record code: put it in main.md + the agent .md files from I5 ("분석·측정 결과는 파일로 만들지 않고 대화창에 출력").
- record/ still saves those answers (record = TUI output, not an agent-made file). confirm with user (open_q 2).

## F. acceptance criteria (lines 96-101) check the plan file, not the feature. replace with:
1 `go build ./... && GOOS=windows go build ./... && go vet ./...` pass
2 `go test ./internal/harness/ -run Record` pass incl. new tests I8, I10, topic-parse (I6), listRecordTopics order across 2 dates + A-B-A
3 test: blocks with fromRecord=true are not written (I12)
4 plain REPL (HARNESS_PLAIN=1) writes the same file as TUI for the same turn
5 `grep -rn "notes/runs" .claude/agents main.md` shows no write instruction left (I5)
6 manual (verify stage only): 2 turns, restart, 3rd turn header `#0003`; `/resume` lists topic; `/resume 1` shows all files in order

## order
S1(I8,I9,I10,I3 slug) -> S3 load funcs (I14) -> S2 recorder + topic (I6,I7,I12) -> S4 /resume (I13) -> S5 plain REPL -> S6 main.md + agents (I4,I5,E) -> S7.
must NOT change: rotate/draining/pendingSnapshot code, existing notes/ content, Claude passthrough of other slash cmds, record_test.go existing 6 tests (extend only).

## open_q (user only)
1 /resume: 화면에 다시 보여주기만 할지, 모델 대화 맥락에도 넣을지 (기본안: 화면만)
2 분석·측정 답변도 record/에 저장할지 (기본안: 저장, 파일 생성만 금지)
3 record/를 git에 넣을지 (기본안: .gitignore 추가, 평문 저장이라 비밀값 노출 위험)
