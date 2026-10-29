# Slash command execution map (read-only scout)
date: 2026-10-29
scope: slashCmds (tui.go:151-172) + unsupportedCmds (repl.go:767); TUI path (default for TTY, runTUI tui.go:1092)
count: slashCmds has 22 entries, not 23. Extra names in unsupportedCmds: /memory, /permissions (not in slashCmds). Table covers all 24 names.

## Dispatch chain (TUI)
- tui.go:641-650  busy: "/usage" -> usageFetch one-shot (tui.go:692 `claude -p /usage --output-format json`); other "/" lines blocked (busySlashMsg)
- tui.go:662-672  idle "/": suggestion pick, then runSlash(line)
- tui.go:853 runSlash: local cases, or submit channel for model-bound cases
- tui.go:1112-1135 submit goroutine: "/rotate"->r.rotate, "/clear"->r.clear, passthroughCmds->r.passthrough, "/import"->importSession, "/mode conductor|conductor-off"->setConductor, "/model <x>"->rotate, default->r.handle
- repl.go:776 passthrough -> runTurn(line) -> repl.go:393 sess.Send (child claude session)
- repl.go:696 handle -> runTurn(text) -> repl.go:393 sess.Send

## Session child (the "claude -p" used for model turns)
- core.go:115 args `-p --input-format stream-json --output-format stream-json ...`; core.go:124 exec.Command(claude)
- Every model-sent row below goes through this child. Local rows that call start() spawn it but send no prompt.

## Table (slashCmds order)
| # | cmd | slashCmds line | handler / dispatch | exec class | claude -p? |
|---|-----|----|----|----|----|
| 1 | /help | tui.go:151 | runSlash tui.go:859 (local note) | LOCAL | no |
| 2 | /clear | tui.go:152 | runSlash tui.go:928 blocks=nil -> submit -> r.clear repl.go:919 | LOCAL (new child, no prompt) | spawns child via start(); no prompt sent |
| 3 | /status | tui.go:153 | runSlash tui.go:873 (local note) | LOCAL | no |
| 4 | /model | tui.go:154 | no args: runSlash tui.go:879 (local note). with args: submit tui.go:1128 -> r.rotate("model x") repl.go:565 | MIXED: no-arg LOCAL; with-arg MODEL-SENT (handoff prompt) | with-arg: child -p runs handoffPrompt (repl.go:585), then new child |
| 5 | /resume | tui.go:155 | runSlash tui.go:889 -> r.sessionList resume.go:66; picker (local) | LOCAL | no |
| 6 | /import | tui.go:156 | runSlash tui.go:899 -> submit tui.go:1120 -> importSession resume.go:82 (clear + applyImport, snapshot sent with next msg) | LOCAL | no (child respawned, no prompt) |
| 7 | /rotate | tui.go:157 | runSlash tui.go:927 -> submit tui.go:1114 -> r.rotate("manual") repl.go:565 | MODEL-SENT (handoff turn, repl.go:585) | yes, via child; skipped when pendingSnapshot set (repl.go:568) |
| 8 | /usage | tui.go:158 | idle: runSlash tui.go:927 -> passthrough tui.go:1118 -> repl.go:776 (child). busy: tui.go:642 -> usageFetch tui.go:692 | MODEL-SENT (passthrough) | yes: child stream-json; busy path = separate `claude -p /usage` one-shot |
| 9 | /cost | tui.go:159 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough) | yes, via child |
| 10 | /context | tui.go:160 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough) | yes, via child |
| 11 | /compact | tui.go:161 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough) | yes, via child |
| 12 | /config | tui.go:162 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough) | yes, via child |
| 13 | /mcp | tui.go:163 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough) | yes, via child |
| 14 | /agents | tui.go:164 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough) | yes, via child |
| 15 | /init | tui.go:165 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough, runs a turn) | yes, via child |
| 16 | /review | tui.go:166 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough, runs a turn) | yes, via child |
| 17 | /doctor | tui.go:167 | runSlash tui.go:927 -> tui.go:1118 -> repl.go:776 | MODEL-SENT (passthrough, runs a turn) | yes, via child |
| 18 | /mode | tui.go:168 | no args / set: runSlash tui.go:905 -> setMode (local). conductor|conductor-off: submit tui.go:1126 -> setConductor repl.go:932 (local toggle) | LOCAL | no (conductor toggle only; classifier.go:115 runs per plain prompt, not for "/" lines, classifier.go:139) |
| 19 | /mouse | tui.go:169 | runSlash tui.go:865 (local, tea mouse) | LOCAL | no |
| 20 | /version | tui.go:170 | runSlash tui.go:924-926 `claude --version` | LOCAL (runs claude binary, not -p) | no (`claude --version` only) |
| 21 | /exit | tui.go:171 | runSlash tui.go:857 -> quit | LOCAL | no |
| 22 | /quit | tui.go:172 | runSlash tui.go:857 -> quit | LOCAL | no |
| 23a | /memory | not in slashCmds | runSlash default tui.go:936-939 note "interactive-only" (unsupportedCmds repl.go:767) | LOCAL (rejected, nothing sent) | no |
| 23b | /permissions | not in slashCmds | same as /memory | LOCAL (rejected, nothing sent) | no |

## Totals (slashCmds, 22)
- LOCAL: 10 (/help /clear /status /resume /import /mode /mouse /version /exit /quit)
- MODEL-SENT: 11 (/rotate /usage /cost /context /compact /config /mcp /agents /init /review /doctor)
- MIXED: 1 (/model: no-arg local, with-arg model-sent)
- Local rows that spawn a child process (start) without a prompt: /clear, /import, /model with arg (plus the handoff turn)

## Caveats
- Plain REPL (non-TTY or HARNESS_PLAIN, repl.go:912-915): loop only special-cases /quit, /rotate, /mode, /resume, /import and passthroughCmds (repl.go:812-846). Everything else, including /help /clear /model /status /version /mouse /exit, falls to handle() (repl.go:696) and is SENT TO THE MODEL. /memory and /permissions get the same treatment there (no unsupported check in the plain loop).
- route() skips "/" lines (classifier.go:139), so the conductor classifier never runs for slash commands.
- Only direct one-shot `claude -p` calls outside the child: tui.go:692 (/usage while busy) and classifier.go:115 (conductor, plain prompts only).

## Not found / not checked
- Did not run anything; read-only.
- Did not verify the child claude's own handling of /compact, /cost etc. beyond the repl.go:763 comment ("tested: stream-json returns their output").
