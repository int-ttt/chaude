# 261101 plan: read output pane = summary + translation only; internal read path English only
user_words: "읽기 출력창에는 요약과 번역물만 출력하도록 수정. 내부 처리는 영어만 사용하도록 재구현."
base: /Users/leekwangseok/PycharmProjects/claude-harness

## current behavior (evidence)
E1: internal/harness/repl.go:603-612 | every main-stream tool_result (incl. Agent return) -> toolResultMsg, no tool-name filter
E2: internal/harness/tui.go:403-411 | toolResultMsg -> dim "result" block = first line trunc 160 + "(+N lines)" -> scout/researcher R-read shows "⎿ status: done … (+N lines)" in pane (caveman line leak)
E3: internal/harness/tui.go:363-370,389-391 | textMsg -> appendAsst, no filter; pane shows main text as-is
E4: internal/harness/repl.go:584-587, tui.go:61-68 | Agent call shows "Agent(<description>)" only; brief text not shown
E5: .claude/agents/haiku-tui.md:11-15 | no R-read render rule; only "Korean 존댓말 natural text only"
E6: .claude/agents/haiku-scout.md:16,20,25-32; haiku-researcher.md:15,17,18-29 | keys English but placeholders Korean ([확실|추정|불확실], <찾았으나 없음 확인> ...), rule line in Korean, "Korean values verbatim"
E7: docs/claude-lean/CAVEMAN_FORMAT.md:8-13,22-27,67 | brief/R-read/header placeholders Korean
E8: plan/caveman-structure.md:56-57, CLAUDE.md:12 | read flow: haiku-tui summary 1~3줄 + items; no English-only rule, no pane filter
E9: .claude/hooks/caveman-format.py:68-74,94-97 | haiku-tui check = 존댓말 only; no caveman-key leak check

## decision
chosen: B = docs (English-only internal read path + haiku-tui read render rule) + TUI code filter hiding caveman tool-result previews + hook leak check
rejected: A docs only: E2 leak stays (code path shows caveman first line regardless of prompts)
rejected: C filter main textMsg lines: textMsg may arrive in chunks; line filter can cut legit text; haiku-tui rule + hook leak check cover it
kept: "Agent(<description>)" tool line; route prefix "[haiku-scout]" (CLAUDE.md:7) -> open question to user

## steps
S1 haiku-batch-worker, docs (exact text in brief):
- .claude/agents/haiku-tui.md: insert read render bullet before Progress render line
- .claude/agents/haiku-scout.md, haiku-researcher.md: Korean rule line -> English; "Korean values verbatim" -> "Korean only inside verbatim source quotes"; R-read placeholders English, tags [sure|likely|unsure]; add English-only line
- docs/claude-lean/CAVEMAN_FORMAT.md: 3.1 lines 8-13 and 3.2 lines 22-27 placeholders English; line 67 reason placeholder English; add rule line after line 31
- plan/caveman-structure.md:56-57 replace; CLAUDE.md:12 replace
S2 haiku-coder, code:
- internal/harness/tui.go: add `func isCavemanBlock(s string) bool` = true iff some line (trimmed) matches `^status:\s*(done|partial|fail|blocked)\b` AND some line matches `^reason:`; toolResultMsg case: if isCavemanBlock(s) do not append block
- internal/harness/tui_caveman_test.go (new): TestIsCavemanBlock table; TestToolResultCavemanHidden (result block count unchanged for caveman, +1 for plain)
- .claude/hooks/caveman-format.py: fmt "tui" also flags "caveman_leak" when a line outside ``` fences matches `^\s*(status|reason|claim|searched|not_found|not_checked|truncated|omitted|summary_ko):\s`
- tests/hooks/test_caveman_format.py: add test_j4 (polite tui text + "claim: a.go:1 | x" -> 1 row with caveman_leak), test_j5 (polite, no keys -> no row)
S3 haiku-runner: go build ./... ; go vet ./... ; go test ./internal/harness/... ; python3 -m unittest discover -s tests/hooks -> counts
S4 haiku-checker: each changed file has the change; read-path files (haiku-scout.md, haiku-researcher.md, CAVEMAN_FORMAT.md 3.1/3.2) have no Korean in templates

## edge cases
- haiku-tui return (Korean first line) still shows as dim result line, then main repeats it (duplicate summary line; not caveman) -> not filtered
- source quotes in Korean stay verbatim inside claim (evidence)
- old tags [확실|추정|불확실] still accepted by haiku-tui
- tool result with harness prefix line before status: -> still hidden (any-line match)
- plain tool output starting "status:" without "reason:" -> still shown

## acceptance
- caveman tool result not appended to pane (Go test)
- haiku-tui caveman key leak flagged (hook test)
- build, vet, go test, unittest PASS with counts
- haiku-checker PASS

## must not change
- caveman key names and key:value shape; hook REQUIRED lists (caveman-format.py:13-17); R-write/R-decide templates; write flow (caveman-structure.md:54-55, CLAUDE.md:11); main-thread-guard; no commits; no file deletions
