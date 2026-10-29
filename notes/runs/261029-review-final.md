verdict: FIX (0 BLOCKING, 3 non-blocking)
build/vet/test: go build ok; go vet ok; go test -count=1 ./internal/harness ok
local cmd path: localCmd (localcmd.go) only formats strings from slashCmds/r fields; no sess.Send; no model call. PASS
issues:
- MED README.md:24,35: still says threshold default 270000; code default now 80000 (repl.go:864) and opus/sonnet fixed 270k. Fix: update both lines.
- MED .claude/agents/sonnet-analyst.md: "no direct file access, Haiku condenses groups of <=3 files" rule only in CLAUDE.md:10; analyst md unchanged and still has Read/Grep/Glob tools (line 4). Fix: add rule line to md (or note CLAUDE.md-only is intended).
- LOW internal/harness/repl.go:690: effectiveThreshold matches Model=="opus"|"sonnet" exactly; full model IDs via --model fall back to Threshold(80k). Fix: strings.Contains/HasPrefix or document.
- LOW repl.go:~822: "/exit" added to plain loop, not in DECISIONS entry. Fix: mention.
- LOW notes/DECISIONS.md:41-42: [rotate] and [local-cmd] lack the quoted-user "->" result form; [local-cmd] says "no code change" yet entry also lists /exit-style changes (localcmd.go, specter test). Fix: reword.
- LOW repl.go metrics (~897) logs a.Threshold only; effective 270k for opus/sonnet not recorded.
ok: agent md final-message wording identical across 8 haiku files; sonnet-analyst/opus-architect/sonnet-reviewer keep old wording (inconsistent with DECISIONS "aligned" claim, LOW).
ok: onToolResultText "Async agent launched" sets Background without Done; onToolResult wrapper preserved; both call sites updated.
ok: tui turnStart/lastTool set at every busy=true site seen.
