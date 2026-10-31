# task: usage/rate source, orch files, OAuth boundary, test fixtures (internal/harness)
# date: 2026-10-31
# scope: read-only; no edits

## 1 usage / 5h rate
usage_cli_poll: usagepoll.go:51 usageRaw (var) = exec claude -p /usage --output-format json --no-session-persistence; Env=cleanEnv(0); Dir=os.TempDir(); timeout usageRawTimeout 60s (:18)
usage_parse: text regex on json.result string. usagepoll.go:21 usageRe `Current session:\s*([0-9.]+)%\s*used(?:\s*·\s*resets\s*([^\n]+))?`; :22 usageWeekRe `Current week \(all models\):...`
usage_parse_fn: usagepoll.go:26 parseUsage -> session_pct, session_resets, week_pct, week_resets; ok requires session line
usage_throttle: usagepoll.go:15 usagePollEvery=5m; :69 pollUsage (env HARNESS_USAGE_POLL=0 disables); :92 pollUsageSync logs stage=usage source=cli
usage_fetch_popup: tui.go:702 usageFetch (same exec, raw string for popup); tui.go:657 call site
rate_source: NO exec, NO HTTP headers. stream event type rate_limit_event -> ev.rate_limit_info (JSON map) from live child stdout: livechild.go:41-42; repl.go:308-309; repl.go:592-593
rate_store: repl.go:215 (r *Repl) setRate -> rateStr, r.rate, lastFiveHour
rate_util: ratestatus.go:22 fiveHourUtil: unifiedWindows.five_hour.utilization, fallback top-level utilization only if rateLimitType ""/five_hour
rate_fmt: ratestatus.go:33 fmtRateStatus -> "HH:MM/HH:MM · N%"; v<1 scaled x100 (:40); resetsAt from unifiedWindows.five_hour or top-level; >1e12 treated as ms (:49)
rate_num: ratestatus.go:57 rateNum (float64|int|string)
rate_diag: ratestatus.go:12 rateRawRow (key names only, no values); repl.go:227 logs once
parse_method: rate = JSON map walk (no regex); /usage = regex on text

## 2 orch.go / orchws.go
orch.go: no process spawn, no transport. mode table only.
orch.go:11 parseOrch (m0|b|c|d|e|f, ""->m0); :23 orchDefaultModel (b,c,d,f->opus; e->haiku); :34 applyOrchEnv (os.Setenv CHAUDE_ORCH); :49 orchRule -> orchRuleB..F consts (:38-46), system-prompt text only
orchws.go: mode f workspace routing. "ws" = workspace id from "[ws:<id>]" prefix, NOT WebSocket (go.mod has no websocket/gorilla dep).
orchws.go:11 wsRe; :14 parseWS; :23 wsState; :31-36 wsMaxTurns=20 wsMaxCtx=40000 wsMaxIdle=270s wsHandoffMax=1500
orchws.go:39 wsNeedsRotate; :47 wsHandoff; :56 orchSwitch (routeChildren or switchSession; defined elsewhere, not read); :69 orchFresh; :78 routeOrch (mode f only; target model haiku; resume st.SessionID or fresh); :118 wsRecord
spawn_actual: core.go:143-154 NewSession: exec.Command(osenv.ResolveBin(ClaudeBin)) args -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model M [--permission-prompt-tool stdio] [--resume ID]; long-lived stdin/stdout, not attached to interactive session
orchws_attach: orchws.go does not spawn; it delegates to r.switchSession / r.routeChildren

## 3 OAuth boundary (non-test *.go, 31 files)
oauth_hits: none for oauth|OAuth|credential|keychain|CLAUDE_CODE_OAUTH|Authorization|bearer|sk-ant|x-api|.credentials.json|login
api_key_hits: core.go:23 and core.go:27 (strip ANTHROPIC_API_KEY from child env)
go_reads_token: NO. Go does not read/store/forward OAuth tokens or credentials. Only removes ANTHROPIC_API_KEY so claude -p keeps subscription login.
token_word_hits (all token-count/cache/pause, not auth) - grep output, cut 100:
budget.go:87:// modelPrice is USD per million tokens. Source: https://docs.claude.com/en/docs/about-
budget.go:89:// sonnet=Sonnet 5.5, haiku=Haiku 5.5 (up to 100k tokens).
budget.go:98:// estimate is the planned cost of the next call: ctx tokens at the cache-read price (w
budget.go:99:// cache-write price for ttl ("1h" 2x input, else 1.25x), plus 4k output tokens. Unknow
budget.go:122:	return (num("cache_read_input_tokens")*p.read + num("cache_creation_input_tokens")*p.
budget.go:123:		num("input_tokens")*p.in + num("output_tokens")*p.out) / 1e6
category.go:9:// categoryRule is appended to the main session's system prompt (<=500 tokens).
clients.go:137:// its last API call is within deadline, and its context is below limit tokens.
core.go:23:// cleanEnv: ANTHROPIC_API_KEY must be absent (with -p a key silently switches billing to
core.go:27:		if strings.HasPrefix(kv, "ANTHROPIC_API_KEY=") {
core.go:73:// ctxTokens: context-size proxy = input + cache_read + cache_creation of one API call.
core.go:74:func ctxTokens(usage map[string]any) int {
core.go:75:	return num(usage["input_tokens"]) + num(usage["cache_read_input_tokens"]) + num(usage["c
core.go:81:	if num(cc["ephemeral_1h_input_tokens"]) > 0 {
core.go:84:	if num(cc["ephemeral_5m_input_tokens"]) > 0 {
core.go:133:	held      chan struct{} // pause token taken by Send, released by Events
classifier.go:180:// classify makes one short Sonnet call; returns model, terse task, cost and token
metrics.go:53:// usageFields extracts token counts from an API usage object; ctx is the context-size
metrics.go:56:		"input_tokens":                num(usage["input_tokens"]),
metrics.go:57:		"cache_read_input_tokens":     num(usage["cache_read_input_tokens"]),
metrics.go:58:		"cache_creation_input_tokens": num(usage["cache_creation_input_tokens"]),
metrics.go:59:		"output_tokens":               num(usage["output_tokens"]),
metrics.go:60:		"ctx":                         ctxTokens(usage),
metricscmd.go:46:					in += tok(d, "input_tokens")
metricscmd.go:47:					out += tok(d, "output_tokens")
metricscmd.go:48:					cr += tok(d, "cache_read_input_tokens")
metricscmd.go:49:					cc += tok(d, "cache_creation_input_tokens")
metricscmd.go:61:							num(d["ctx"]), tok(d, "input_tokens"), tok(d, "output_tokens"),
metricscmd.go:62:							tok(d, "cache_read_input_tokens"), tok(d, "cache_creation_input_tokens"),
metricscmd.go:73:						fmt.Printf("%s,rotate,%s,,%d,,,,,,,,,%v\n", str(d["ts"]), str(d["old_session"
metricscmd.go:90:	fmt.Printf("tokens: input=%d output=%d cache_read=%d cache_write=%d\ncost_usd=%.4f
metricscmd.go:94:// tok reads a per-turn token field, preferring the turn total ("result_" prefix) o
orch.go:38:const orchRuleB = "[orch B] Opus main decides inline. ALL file reads/writes go through ha
orch.go:46:const orchRuleF = "[orch F] Turns prefixed [ws:<id>] belong to one workstream; the harnes
repl.go:33:	preamble = "[Context carried over from a previous session that was rotated to save token
repl.go:37:		"Log each delegation (agent, tokens, duration, outcome, retries)."
repl.go:419:		kv{"ctx", ctxTokens(st.usage)}, kv{"cost", cost}, kv{"source", "background"})
repl.go:659:	ctx := ctxTokens(usage)
repl.go:677:		dim, commas(ctx), num(ru["input_tokens"]), commas(num(ru["cache_read_input_tokens"])),
repl.go:678:		commas(num(ru["cache_creation_input_tokens"])), num(ru["output_tokens"]), cost, five, 
repl.go:820:		"reco_preserved": reco, "attempts": attempts, "outcome": how, "rotate_at_tokens": at,
repl.go:859:// effectiveThreshold is the context-token count that triggers auto-rotation for the cur
repl.go:916:		kv{"ctx", ctxTokens(usage)}, kv{"cost", cost})
repl.go:1068:	fs.IntVar(&a.Threshold, "threshold", 150_000, "rotate after an answer when context tok
tui.go:156:	{"/status", "model, context tokens, session"},
tui.go:240:		dimSt.Render(fmt.Sprintf("  model: %s · rotate at %s tokens", m.r.a.Model, commas(m.r.e
turnspend.go:3:// turnSpend accumulates subagent token usage and Agent/Task spawns seen in one turn'
turnspend.go:14:var spendKeys = []string{"input_tokens", "output_tokens", "cache_creation_input_toke
## 4 tests
test_files: 38 (*_test.go under internal/harness)
fake_claude_binary: none; no test runs a fake claude on PATH; resume_test.go:42,47 only string cases
in_process_fakes: idle_test.go:14 fakeSession (channels); children_test.go:27 fakeCloser; budget_test.go:179 fakeCmdWithPID; usagepoll_test.go:68-69,85-86 stub usageRaw var; usagepoll_test.go:15-16 liveUsageSample text
real_exec_in_tests: acceptance_test.go:25,27,35 go build ./cmd/chaude then runs built binary repl
testdata/: bg_turn_live.jsonl (124110 bytes, 2026-10-30) only; used by idlespend_test.go:85 (t.Skipf if missing). No fake claude binaries/fixtures dirs.
not_found: no websocket dep; no OAuth/credentials handling in Go; switchSession/routeChildren bodies not read
