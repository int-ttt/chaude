# review: 261031-plan-model-policy-rewrite.md (opus-architect, design only, not executed)

verdict: revise before S0. core direction ok (router out, haiku main -> opus-architect, 80k/200k). 3 blockers, 6 fixes.

## blockers
- B1 S0/O1 base conflict: worktree already holds router removal (git diff --stat 15 files +119/-867; DECISIONS.md:47; notes/runs/261031-haiku-model-policy-impl.md). "branch from HEAD, dirty archived not reused" needs restore/checkout over dirty files = forbidden by §5/§7. fix: S0 = archive patch (keep), then `git switch -c model-policy-rewrite` (carries worktree), run go build/vet/test, commit worktree as baseline "S0 baseline (prior impl)". later steps edit on top. drop redo of deletions already done.
- B2 sonnet conflict: P4, S6 restore sonnet-reviewer.md, S10 sonnet-reviewer contradict DECISIONS.md:47 ("sonnet tier removed; sonnet-* agent files and sonnet pin logic removed"). fix: do not restore; S10 review = opus-architect (design) + haiku-checker (diff/tests). agent-model-pin.py: no sonnet allow rule. drop O13. if user wants sonnet-reviewer back -> open_q, not default.
- B3 forward.go contradicts DECISIONS.md:47 ("no Go opus wrapper"). forwarding is done by main Haiku model via Agent tool, not Go text rewrite. fix: delete forward.go/forward_test.go from CREATE. replace conductorRule (repl.go:1209) with const forwardRule in policy.go (system-prompt text: forward verbatim + 1 summary line + lang: ko to opus-architect; relay summary_ko via haiku-tui; local cmds and acks not forwarded, ack list = 네/ㅇㅇ/고마워/ok). test: policy_test.go asserts forwardRule contains "opus-architect", "lang: ko", "summary_ko".

## fixes
- F1 O7 wrong default: DECISIONS.md:47 keeps "-threshold flag (default 80,000)". keep flag (repl.go:1069) default haikuThreshold; opus fixed 200000 (repl.go:865 already). do not remove.
- F2 conductor_test.go:27-61 TestThresholdFor already asserts 80000/200000. move it to policy_test.go (rename), do not archive-and-rewrite.
- F3 missing step: .claude/settings.json autoCompactWindow 150000 < opus 200000 (DECISIONS.md:47 open) -> opus rotation unreachable, compaction fires first. add to S5: set autoCompactWindow > 200000 (e.g. 220000) or note why not; child compact-window 400k stays > threshold (DECISIONS.md:14).
- F4 move O10 check to S1: scout reads .claude/hooks/agent-model-pin.py and confirms main -> opus-architect Agent call allowed BEFORE any code step. blocked -> stop, report, no bypass. whole plan depends on it.
- F5 scope creep / duplicate: v0.9.0 (commits 27c60e2, 0ee3501) already merged caveman S0~S5; haiku-tui agent already registered. S1 scout must `ls` docs/claude-lean/CAVEMAN_FORMAT.md, .claude/agents/haiku-tui.md, .claude/hooks/caveman-format.py, tests/hooks/test_caveman_format.py and grep R-read blocks in haiku-*.md. existing -> drop from S6/S7. only gaps stay.
- F6 S3 then S4 leaves broken build per commit. merge S3+S4 into one commit (delete + fix callers), or commit only after build PASS.

## small
- §5 S8 :154 leftover text "haiku-drafter? NO" -> remove. :155 "caveman S5 (O1)" -> "(O12)".
- CHAUDE_ROUTER_MODEL is correct (binary chaude, DECISIONS.md:47); keep.
- acceptance grep "conductor" will hit renamed tests/history; scope grep to identifiers: `Conductor|setConductor|conductorRule|parseOrch|orchDefaultModel|CHAUDE_ROUTER_MODEL`.
- O4 (opus로 override) and opus-threshold scope tie together: if main fixed haiku and no /model opus switch, opusThreshold 200k never applies. keep REPL model switch (session mgmt) so 200k has a target; state it in P1.

## open_q (user only)
- Q1 B2: sonnet-reviewer 복원 여부 (DECISIONS.md:47은 sonnet 전면 제거).
- Q2 opus 200k 적용 대상: REPL에서 /model opus 전환 세션인지 확인 필요.
