# trace: is every user input handled by a Haiku process (2026-10-29)
task: read-only trace, no code edits. repo: /Users/leekwangseok/PycharmProjects/claude-harness

## defaults
- --model default: "haiku" repl.go:862; --conductor default true repl.go:879
- NewSession empty model -> haiku core.go:109-110; argv "--model <m>" core.go:116
- start() passes r.a.Model to child repl.go:171,183
- conductorModel = "sonnet" classifier.go:14; classifier argv "--model sonnet" classifier.go:115

## entry points
- TUI submit: tui.go:677 sendPrompt -> tui.go:716 submit chan -> goroutine tui.go:1126-1149 switch
- plain loop: repl.go:795-857 (ReadString -> localCmd/passthrough/handle)
- handle: repl.go:696; route only if Conductor repl.go:700-701
- route: classifier.go:138; returns raw line for isAck or "/" prefix classifier.go:139-141
- isAck: category.go:22-25 (empty, ackWords, <=2 runes)

## per input type
1. normal prompt (TUI): tui.go:677 -> 1147-1148 handle -> route -> classify (Sonnet, prompt text in stdin classifier.go:114) -> runTurn repl.go:393 to main child (Haiku default). PROCESS: Sonnet classifier + Haiku main.
   - pickModel classifier.go:38-46: cur haiku/"" -> haiku; cur opus -> opus (delegate if topic new & size small); cur sonnet -> sonnet. Classifier output never upgrades main from haiku.
   - haiku worker reuse: switchSession("haiku", worker) classifier.go:165-167,193-213 (haiku only)
   - size large + haiku main -> planHint classifier.go:94-95 (tells Haiku to call opus-architect / sonnet-analyst subagents)
   - classifier error -> raw line to main classifier.go:153-156
   - classifier failure/ack: raw text to main (Haiku)
2. normal prompt (plain): repl.go:853 handle -> same as above.
3. queued prompts while busy: tui.go:656-661 queue; flush tui.go:433-436 sendPrompt -> classifier (merged text).
4. conductor off (/mode conductor-off): tui.go:1142-1143 -> setConductor repl.go:936-938; repl.go:700 skipped -> no classifier, main Haiku.
5. local slash (TUI): runSlash tui.go:869-960: /help 875, /mouse 881, /status 889, /model no-arg note 895-899, /resume picker 905-914, /mode no-arg/arg 921-939, /version 940-942 (claude --version, no model). No child, no classifier.
6. local slash (plain): localCmd localcmd.go:10-22 (/help, /status only). /resume repl.go:~838 resumeLine; /import importSession resume.go:82.
7. passthrough (/cost /context /compact /config /mcp /agents /init /review /doctor /usage idle): TUI tui.go:943-951 -> 1134-1135 -> passthrough repl.go:770-793 -> runTurn -> child (Haiku default). NO classifier (route not called). Plain: repl.go:849-850.
   - /usage busy: tui.go:645-649 -> usageFetch tui.go:694-696 `claude -p /usage` WITHOUT --model -> user default model (not pinned). EXCEPTION.
8. /model X (TUI): runSlash tui.go:895-904 -> submit -> tui.go:1144-1146: r.a.Model = X (unvalidated); r.rotate("model X") repl.go:565. Handoff from OLD child repl.go:585 (old model); new child started with X repl.go:634 -> X passed as --model core.go:116. No classifier. EXCEPTION: X can be opus/sonnet = forced non-Haiku main; pickModel keeps it (classifier.go:45).
   - /model no-arg: local note tui.go:895-899.
   - plain loop: /model not special-cased -> repl.go:853 handle -> route returns line (classifier.go:139) -> sent as text to current child repl.go:393. No model switch. EXCEPTION (no switch happens).
9. /rotate: TUI tui.go:1130-1131 / plain repl.go:824 -> rotate repl.go:565 -> handoff runTurn repl.go:585 on OLD child (Haiku default) -> start() new child on r.a.Model repl.go:634. No classifier.
10. /clear: TUI tui.go:1132-1133 -> clear() repl.go:923-934 -> start() on r.a.Model; no model call. Plain loop: NOT handled; repl.go:853 -> sent as user text to child. EXCEPTION.
11. /import, /resume (TUI picker): tui.go:915-920 / 1136-1141 -> importSession resume.go:82-99 -> r.clear() resume.go:92 -> start() on r.a.Model. No classifier.
12. /mode conductor|conductor-off: no model call (tui.go:1142-1143; repl.go:936).
13. classifier call itself: classify classifier.go:109-134, cmd classifier.go:115 `claude -p ... --model sonnet --tools "" --system-prompt ... --no-session-persistence`, cwd os.TempDir (117). Called once per non-ack non-slash prompt when conductor on (classifier.go:147). Cost logged metrics stage=classify (repl.go / classifier.go:149-163).
14. subagents (Task tool, run by main child, not harness-spawned): model from .claude/agents frontmatter: haiku-* -> haiku; sonnet-analyst.md:5, sonnet-reviewer.md:5 -> sonnet; opus-architect.md:5 -> opus. Reached when Haiku main delegates (delegationRule repl.go:34; delegateHint classifier.go:27; planHint classifier.go:31-33).
15. auto rotation / category rotation: rotate repl.go:565 (handoff on current child, new child on r.a.Model). No new model.

## no model call at all
- /help /status /mouse /mode(no arg or set) /quit /exit /version(claude --version only) /resume (list) /model(no arg) /mode conductor toggles
- empty lines and ack/slash lines skip classifier (classifier.go:139) but still reach main child only if not local

## exceptions (non-Haiku or no-model paths)
- E1 Sonnet classifier: every normal non-ack prompt when conductor on (classifier.go:115, default true repl.go:879). Sees full prompt text (classifier.go:114).
- E2 forced main: --model sonnet|opus at start (repl.go:862) or /model sonnet|opus (tui.go:1144-1146). Classifier keeps it (classifier.go:45).
- E3 Opus main delegate: cur opus + new + small -> delegateHint (classifier.go:43, 27).
- E4 subagents opus-architect/sonnet-*: reached via Haiku main when size large (classifier.go:94-95, 31-33).
- E5 /usage while busy: unpinned `claude -p /usage` (tui.go:695) -> default model of user's Claude Code config.
- E6 plain-mode /model and /clear: sent as text to current child, no model change, no local handling (repl.go:849-853).
- E7 /model X accepts any string verbatim (tui.go:1145, core.go:116).

## not verified
- Claude CLI behaviour for stream-json "/clear" or "/model" text (whether child interprets them as builtins): not checked.
- Model used by the default `claude` when no --model (E5): depends on user's Claude Code settings; not in repo.
- Compiled ./chaude binary not compared with source (only source read).
