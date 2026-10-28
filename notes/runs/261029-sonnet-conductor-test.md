# sonnet conductor (v0.6.7) - DONE
impl: --conductor flag + "/mode conductor|conductor-off" (plain+TUI); Sonnet main + conductorRule (repl.go): conductor outputs ONLY Agent delegation (task+target), never answers; terse machine-style prompt; judgment/pushback/correction/strategy -> opus-architect; category rotation kept; metrics field "conductor". test conductor_test.go.
decisions: notes/DECISIONS.md [conductor] x2 (user corrections 261029)
A/B (1 run, tiny task "Test 함수 개수", no --auto-allow since classifier denied it): opus main $0.2517 (answered via haiku-scout, correct: 1) vs sonnet conductor $0.1230 main-only (delegated to haiku-scout; stdin closed before agent result, so subagent cost + final answer not captured => not comparable, n=1). Source: metrics stage=turn cost_usd.
release: commit 3144c13, tag v0.6.7, Actions success, installed ~/.local/bin/chaude -> "chaude 0.6.7". master merged into main (ff-free merge, pushed).
doubts: A/B incomplete; conductor deleg-only behavior verified only by one run.

## full A/B (3 read-only tasks, stdin kept open until quiet 45s; driver $TMPDIR scratchpad drv/ab.py; metrics sum of stage=turn cost_usd, incl. background turn)
tasks: T0 category.go 함수 수 / T1 main.go version 값 / T2 harness _test.go 수
opus main: T0 $0.1505 (1 turn) / T1 $0.1451 / T2 $0.1453; ctx ~29k per turn (cr ~39k cc ~16k out ~330); final reply arrived 3/3; all via haiku-scout, correct (5 / 0.6.7 / 14)
conductor: T0 $0.1731 (2 turns) / T1 $0.1518 / T2 $0.1520; turn1 ~ $0.07-0.08 (cr ~37-40k cc ~17-19k out ~245-328), turn2 relay ~ $0.08 (cr ~29k cc ~0.7k out ~46-99); final reply arrived 3/3; correct
total: opus $0.4409 vs conductor $0.4769 (+8%); spend of this A/B ~$0.92
conductor-only-delegates check: FAIL. It delegates to haiku-scout but (a) writes interim prose ("확인하는 중입니다...") and (b) after the agent result it composes the final Korean reply itself + [cat:] marker, i.e. answers. Opus main behaved the same shape (haiku-scout, 1 turn).
conclusion: no cost win on tiny tasks (Sonnet main is not cheaper per turn here, extra relay turn); n=3, tiny, cache-dominated; tokens column in my driver was 0 (wrong keys), numbers from ctx lines.
open: relay turn is inherent to design (agent result returns to conductor); to meet "never answers" the harness would have to display agent output directly instead of the conductor re-reply; needs user/opus decision.

## classifier rework (v0.6.7 not yet tagged) + A/B rerun (same 3 tasks, driver keeps stdin open)
impl: classifier.go (classify/parseRoute/Repl.route), handle() calls route() when --conductor; /mode conductor just toggles; conductorRule removed. test TestParseRoute. gofmt/vet/build/test -race ok.
opus main: T0 $0.1192 / T1 $0.1147 / T2 $0.1153 (sum $0.349); answered itself from memory without rechecking (said "이번에 다시 열어보지 않았습니다")
classifier -> haiku session: T0 $0.0852 (classifier $0.0775 cold cache + session $0.0077) / T1 $0.0157 (cls $0.0065, 2 turns: interim + background result) / T2 $0.0113 (cls $0.0065); routed haiku x3; sum $0.112 (-68%); correct answers 5 / 0.6.7 / 14, replies arrived 3/3
caveats: n=3; T0 classifier cost inflated by cold cache (later calls ~$0.0065); opus did not re-verify so quality not equal; haiku replies add "추천:" blocks (verbose); T1 had an interim reply then final. Cost = metrics stage=turn + stage=classify cost_usd, A/B spend ~$0.46.
