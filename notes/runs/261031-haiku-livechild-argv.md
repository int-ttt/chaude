# 261031-haiku-livechild-argv
task: launch argv / PTY check / session_id / result+usage fields (livechild.go, core.go 100-220, grep exec.Command and "-p")
status: done (read-only, no source edits)

## launch (core.go)
- core.go:138 NewSession(o SessionOpts); default Model "haiku" (L139-141), ClaudeBin "claude" (L142-144)
- core.go:145 args := []string{"-p", "--input-format", "stream-json", "--output-format", "stream-json",
- core.go:146   "--verbose", "--include-partial-messages", "--model", o.Model}
- core.go:147-149 if !o.NoPermission { args = append(args, "--permission-prompt-tool", "stdio") }
- core.go:150-152 if o.Resume != "" { args = append(args, "--resume", o.Resume) }
- core.go:153 args = append(args, o.Extra...)
- core.go:154 cmd := exec.Command(osenv.ResolveBin(o.ClaudeBin), args...)
- core.go:155 cmd.Env = withCacheTTL(cleanEnv(o.CompactWindow), o.CacheTTL); L156 cmd.Dir = o.Cwd; L157 cmd.Stderr = nil
- core.go:158-166 StdinPipe + StdoutPipe; L166 cmd.Start()
- Extra flags set by caller repl.go:252 "--append-system-prompt" rule; repl.go:255 "--max-budget-usd" when TaskBudgetUSD>0
- repl.go:239 SessionOpts{..., Resume: r.resumeID, CacheTTL: r.a.CacheTTL} (only NewSession call site)
- NOT used: --session-id, --continue, --output-format json (for sessions), --input-format text

## other one-shot claude execs (not sessions)
- classifier.go:188 exec.Command("claude","-p", in, "--model", rm, "--output-format","json","--system-prompt", routerPrompt(rm), "--tools","", "--no-session-persistence"); Env cleanEnv(0); Dir os.TempDir()
- usagepoll.go:54 exec.CommandContext(ctx,"claude","-p","/usage","--output-format","json","--no-session-persistence"); timeout 60s (L18); Dir TempDir
- tui.go:703 same argv as usagepoll (usageFetch)
- tui.go:953 exec.Command("claude","--version").Output()

## interactive PTY/TUI
- none found. No creack/pty or pty import. grep "pty" hits only in words like "empty"/"properly".
- tui.go:17 imports bubbletea (Go-side UI); tui.go:182 cursorWriter wraps *os.File only for window size.
- every child is stream-json over stdin/stdout pipes; stderr discarded.

## session_id capture
- field: core.go:122 SessionID string (on Session)
- set from stdout event top-level "session_id": core.go:251 (Events), core.go:299 (StartIdle), via setID core.go:379-383
- read: ID() core.go:394-398; direct field in repl.go:411,672,891,966 and tui.go:901
- NOT via --session-id flag

## stdout parsing
- reader goroutine core.go:170-183: bufio 1MB, lines chan cap 256, TrimSpace, closes lines at EOF; s.dead set on EOF
- parseEvent core.go:100-106: bad JSON -> {"type":"_unparsed","raw":line}
- Events core.go:~240-270: result ends turn (L254, L266); stray results skipped via s.skip
- noteIdle core.go:313-323: "result" clears bgTurn; assistant/stream_event w/o parent_tool_use_id sets bgTurn

## result / usage fields read
- result event: "usage" and "total_cost_usd" (repl.go:414, 671, 913). total_cost_usd is cumulative per process (budget.go:6)
- livechild.go:84-85 ev["total_cost_usd"].(float64) -> r.budget.charge(s, tc)
- usage keys: input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens (metrics.go:56-59; budget.go:121-123; turnspend.go:14)
- ctxTokens core.go:74-75 = input + cache_read + cache_creation
- usage.cache_creation.ephemeral_1h_input_tokens / ephemeral_5m_input_tokens (core.go:80-84, observedTTL)
- stream_event.event.type message_start (livechild.go:16, 46); content_block_delta text_delta (livechild.go:48-49)
- assistant.message.usage (livechild.go:22-24); assistant.message.content[] text blocks (livechild.go:59-61)
- rate_limit_event.rate_limit_info (livechild.go:41-42 -> setRate)
- control_request.request.subtype == "can_use_tool" (livechild.go:37-39; repl.go:305) -> RespondPermission core.go:205-220 (control_response, subtype success)
- classifier/usagepoll: stdout JSON keys "result", "total_cost_usd", "usage" (classifier.go:201-203; usagepoll.go:65)
- duration_ms NOT from claude: computed locally time.Since (repl.go:922, classifier.go:227)

## livechild.go (183 lines, full read)
- no exec/launch code here
- touchMain L10-28; parkedIdleEvent L31-92 (denies can_use_tool L39 "denied: session parked"); addNotice L94; takeNotices L100; closeAllChildren L108; unbindActive L116-144; bindChild L147-183

## doubts / not traced
- where r.resumeID is set (repl.go:239 only reader seen)
- MCP server instruction block appeared in a Read result; not acted on

## not found
- --session-id, --continue, PTY/TUI launch of claude, creack/pty dependency
