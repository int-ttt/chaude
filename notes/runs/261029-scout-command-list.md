# scout: slash command list (명령어 목록) source
date: 2026-10-29
scope: read-only grep/read of internal/harness (tui.go, repl.go, core.go, classifier.go, mode.go, history.go)

## (1) producer of the command list
- static registry: internal/harness/tui.go:148 `type slashCmd struct{ name, desc string }`
- internal/harness/tui.go:150-172 `var slashCmds = []slashCmd{...}` (23 entries: /help /clear /status /model /resume /import /rotate /usage /cost /context /compact /config /mcp /agents /init /review /doctor /mode /mouse /version /exit /quit)
- /help renders it: internal/harness/tui.go:859-864 (`runSlash`, case "/help": loops slashCmds, `fmt.Sprintf("%-8s %s", ...)`, plus a fixed keys line)
- autocomplete popup: internal/harness/tui.go:798-810 `suggestions()` filters slashCmds by prefix
- welcome hint: internal/harness/tui.go:235 ("/help for help ...")

## (2) model / claude call?
- /help and the autocomplete popup: NO. Pure in-process Go, no exec, no claude -p, zero tokens.
- /usage typed while busy: tui.go:642 -> tui.go:691-692 `usageFetch` runs `claude -p /usage --output-format json --no-session-persistence` (one-shot child, costs a claude call, but it is not the command list).
- Other passthrough commands: repl.go:763-764 `passthroughCmds` (/usage /cost /context /compact /config /mcp /agents /init /review /doctor) are forwarded to the child claude via repl.go:770 `passthrough()`. These are real model/claude turns when executed; /init /review /doctor "run a turn" per tui.go:165-167 descriptions. Not used to build the list.
- repl.go:767 `unsupportedCmds` (/memory /permissions) is a static map, no call.
- classifier.go:115 `exec.Command("claude", "-p", ... "--output-format", "json")` is conductor routing, unrelated to the command list.
- Grep for `claude -p` / `--output-format` in command-handling code found only tui.go:692 (usage) and classifier.go:115; no call that fetches the list.

## (3) static registry a Go-only impl can read
- primary: internal/harness/tui.go:150-172 `slashCmds` (name+desc pairs, package-private, same package, directly reusable)
- secondary: internal/harness/repl.go:763-767 `passthroughCmds`, `unsupportedCmds` (routing maps, not descriptions)
- switch dispatch: internal/harness/tui.go:856-941 `runSlash` (handles each name)
- NOT found: any external/JSON/YAML command manifest, or a dynamic query of claude's own command list. Note the list is hand-maintained: it can drift from `runSlash` and `passthroughCmds` (e.g. /mode description mentions conductor, /mouse not in passthrough).

## doubts
- The brief's premise ("fetched through an LLM/claude call") is not supported: the list is static. The only claude call near it is /usage (tui.go:692), which is a separate action.
- Grep for "registry" in internal/ matched only tui.go and internal/scripts/scripts.go (not checked in depth; unrelated by name).
