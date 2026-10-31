# scope: v0.8.1 (resume + output mode + sonnet no-direct-read + release)
status: ESCALATE, nothing implemented
why:
- 5+ interlocked files: repl.go, tui.go, drain.go (resume), CLAUDE.md, .claude/agents/sonnet-analyst.md, haiku-context-guard.py (+settings.json?), CHANGELOG/version
- worktree already dirty: internal/harness/{agentview,children,classifier,drain,livechild,repl,tui}.go + tests + notes/FACTS.md modified, uncommitted. Unknown whether these are the intended resume/0.8.0-follow-up edits; releasing would bundle them.
- conflict: brief says "final results in TUI, not files" but my standing instructions require answers in notes/runs/ and only path returned. Needs main-thread decision on which governs.
- conflict: commit trailer. brief: "Co-Authored-By: Claude Haiku 5.5"; harness attribution reminder: "Claude Sonnet 5.5". Needs explicit choice.
- LESSONS.md: resume fix failed once (draining session rejected it) -> design (import snapshot vs rotate overwrite, TUI block) must be settled before coding.
- tag+release is irreversible/outward: do only after reviewer approval and user-confirmed trailer.
options:
 1. split into 3 workstreams: (A) sonnet no-Read: edit sonnet-analyst.md:18 + deny in haiku-context-guard.py:22 (small, 2 files); (B) output mode per analysis rec #2 (CLAUDE.md L5/L15/L20, repl.go:839, tui.go:400/435); (C) resume (drain.go/repl.go/tui.go). Then reviewer, then release.
 2. A first now (low risk), B/C after decision on dirty tree.
evidence: notes/runs/261029-analysis-next-items-and-output-mode.md, 261029-analysis-sonnet-direct-read.md, notes/LESSONS.md
open: which trailer; commit dirty tree as-is?; output-mode vs notes/runs rule.

## design change (user order 261029): opus-architect no direct file access
change: .claude/agents/opus-architect.md tools `Read, Write, Edit, Bash, Grep, Glob, Agent` -> `Agent`
why: opus was reading/editing files directly; user wants it limited to design+delegation
effect: all file read/search -> haiku-scout (or sonnet-analyst); edits -> haiku-coder/haiku-batch-worker; notes/runs writing -> haiku-drafter/sonnet-analyst; verify -> haiku-checker
note: Bash/Grep/Glob also removed (Bash=cat bypass; Grep/Glob read contents)
risk: opus cannot self-read ESCALATE scope files; main must quote them in brief or scout reads them
rule (CLAUDE.md, added): Opus never reads/writes files directly; reads -> haiku-scout, writes -> haiku-coder/haiku-batch-worker (docs -> haiku-drafter/sonnet-analyst)
escalate path: main gives ESCALATE path to opus-architect; architect has scout read it (no direct Read)
v0.8.1 conflict check: workstreams A/B/C (sonnet no-Read, output mode, resume) are all executed by Sonnet/Haiku agents -> no conflict; only plan files in notes/runs must be written via delegates, not by opus
