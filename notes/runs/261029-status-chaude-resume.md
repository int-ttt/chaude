# status-chaude-resume (261029)
Paths relative to internal/harness/ unless noted. Read-only check; nothing edited. Tests not re-run by me.

## Release status (evidence)
- notes/runs/release-rebuild.md:1-7: v0.7.1 vet+test PASS, sonnet-reviewer FIX applied, pushed, release workflow success, install via gh PASS (curl install.sh 404, private repo).
- Commits: 18f85d2 (0.7.1 code+notes), b6e057f (metrics only); 3714a39 (0.7.0 persist subagent state). Working tree: notes/metrics/2026-10-29.jsonl modified, release-rebuild.md untracked (git status at start).
- Deferred, not fixed (release-rebuild.md:4): haiku->haiku worker switch not parked; usageFetch no timeout; pickTarget unused kind param.
- notes/runs/haiku-conductor-plan.md:4: unverified - live classifier (CHAUDE_LIVE=1) not run; Haiku following planHint not tested.
- notes/FACTS.md is empty (only header); durable decisions are in notes/DECISIONS.md (38 lines).
- DECISIONS.md:15: "/memory /permissions /resume unsupported (hidden)" (261028) - now stale: /resume is listed in tui.go:155 and handled tui.go:889, repl.go:822.

## /resume: what it loads
- IMPLEMENTED, but source is ONLY Claude Code's own store, not a chaude store.
  - resume.go:20-30 claudeProjectDir -> ~/.claude/projects/<cwd with / . _ -> ->.
  - resume.go:33-50 listClaudeSessions globs *.jsonl there; resume.go:58-64 resumeSourceMsg: no arg or "claude" accepted, anything else -> usage "lists Claude Code conversations only".
  - resume.go:66-79 sessionList; resume.go:82-99 importSession -> resolveClaudeTranscript (importclaude.go:22-50) + readClaudeTranscript (importclaude.go:53) -> applyImport.
  - Entry points: plain loop repl.go:822-823, TUI tui.go:889-899 (picker), CLI flag --from-claude repl.go:865 / importFromClaude importclaude.go:128.
- So the claim "/resume loads only chaude's own history store" is FALSE in code: it loads only Claude's store. /resume claude is the same as bare /resume (added as a filter/arg check in 18f85d2, resume.go diff).

## "/resume claude" = Claude history load + migration into chaude store
- Load: IMPLEMENTED (above). It parses turns/files/last 추천 into memory (importclaude.go:151-153: r.tr.turns, r.files, r.lastReco) and sets r.pendingSnapshot, sent with the next message (resume.go:97). It does not use claude --resume for the imported session; the history is re-fed as a text snapshot.
- Migration into a persistent chaude store: MISSING. No code writes imported turns to any chaude conversation store. Writes found (grep WriteFile/OpenFile): agentstore.go:169 (~/.chaude/agents.json, sub-agent state only, path agentstore.go:255), repl.go:89 (Transcript.add, append-only event log at StateDir/transcript.jsonl, repl.go:851), metrics.go:38 (notes/metrics/*.jsonl). applyImport only logs a metrics "import" stage (importclaude.go:160).
- Chaude's "own history" that exists: transcript.jsonl event log (repl.go:80-95) - write-only; nothing reads it for /resume. Input prompt recall (history.go) is in-memory only.
- Restore side effect: applyImport calls restoreAgents(sessionID) (importclaude.go:159; agentstore.go:230-237), so sub-agent state saved in ~/.chaude/agents.json is restored by Claude session id (0.7.0, 3714a39).
- Design-level: no DECISIONS.md line defines a chaude-own history store or migration (grep history/store/migrat found none relevant). Whether one is wanted is an open decision.

## Not verified
- Did not run tests/binary; did not check /resume at runtime. Did not read tui.go picker rendering beyond line refs. Did not grep sources outside /Users/leekwangseok/PycharmProjects/claude-harness (chaude Go source is in this repo: cmd/chaude, internal/harness). Reviewer's release-review.md not read.
