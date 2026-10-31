# run: 261031-haiku-usagepoll-readout
task: read usagepoll.go, ratestatus.go, turnspend.go, metrics.go (internal/harness) and summarize data sources, funcs, field meanings
status: done (read-only)
scope-note: no URL/endpoint and no ~/.claude path and no anthropic-ratelimit-* header appear in these 4 files

## usagepoll.go (126 lines)
- source: `claude -p /usage --output-format json --no-session-persistence` (one-shot CLI subprocess, not HTTP). cmd.Env=cleanEnv(0), cmd.Dir=os.TempDir(). Reads JSON field "result" (text).
- const usagePollEvery = 5*time.Minute ; usageRawTimeout = 60*time.Second
- usageRe  = `Current session:\s*([0-9.]+)%\s*used(?:\s*·\s*resets\s*([^\n]+))?`
- usageWeekRe = `Current week \(all models\):\s*([0-9.]+)%\s*used(?:\s*·\s*resets\s*([^\n]+))?`
- func parseUsage(text string) (map[string]any, bool)  ; ok requires session line. keys: session_pct (float), session_resets (opt str), week_pct (float, opt), week_resets (opt str)
- var usageRaw = func() (string, error)
- func (r *Repl) pollUsage()  ; disabled if env HARNESS_USAGE_POLL=="0" or r.metrics==nil ; throttled by usagePollEvery ; one goroutine at a time (usagePolling flag). Called after each turn (repl.go:442).
- func (r *Repl) pollUsageSync()  ; row on success: stage="usage", source="cli", session_pct, week_pct... ; error row: {"stage":"usage","error":<err>} ; parse error row: {"stage":"usage","error":"parse","head":<first 120 runes>}
- func (r *Repl) turnCostUSD(sess string, cost any) (float64, bool)  ; per-turn cost = cumulative total_cost_usd delta within a session ; resets to 0 on new session or when cost decreases ; becomes fields["cost_turn_usd"] (repl.go ~line 425)

## ratestatus.go (68 lines)
- source: stream-json event key "rate_limit_info" (livechild.go:42 ; repl.go:309, repl.go:593 -> r.setRate(asMap(ev["rate_limit_info"])))
- func rateRawRow(info map[string]any) map[string]any : {"stage":"rate_raw","keys":<sorted key names>,"rate_limit_type":str} ; key names only, no values; logged once per process (r.rawRateLogged)
- func fiveHourUtil(info map[string]any) (float64, bool) : info["unifiedWindows"]["five_hour"]["utilization"] ; fallback info["utilization"] only when info["rateLimitType"] is "" or "five_hour". Raw scale (not normalized).
- func fmtRateStatus(info map[string]any, now time.Time) string : returns "HH:MM/HH:MM · N%" = now / reset time · rounded percent. If util<1, multiplied by 100 (fraction scale). reset from unifiedWindows.five_hour.resetsAt (fallback top-level resetsAt under same rateLimitType rule); value >1e12 treated as ms. Returns "" when no util.
- func rateNum(v any) (float64, bool) : accepts float64, int, string (ParseFloat).
- setRate (repl.go:214) logs {"stage":"rate","five_hour":v} only when five_hour value changes (raw scale). Status bar text uses unifiedWindows.five_hour.utilization via fmt.Sprint (repl.go:666).
- NOT present in these files: 7d / weekly rate window from stream; week value only comes from /usage text (week_pct).

## turnspend.go (84 lines)
- type turnSpend { sub map[message.id]map[field]float64 (per-field MAX across streamed partials) ; tools map[tool_use id]bool ; types map[subagent_type]int }
- func newTurnSpend() *turnSpend ; created per turn (repl.go:299, 504)
- var spendKeys = input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens
- func (s *turnSpend) onEvent(ev event) : only type=="assistant". Sub-agent event = !isMainEvent (parent_tool_use_id non-empty). Takes message.id + message.usage. tool_use blocks named "Agent" or "Task" counted once by block id; subagent_type from input, default "general-purpose".
- func (s *turnSpend) fields() map[string]any : sub_msgs (distinct sub message ids) ; sub_<key> = sum over sub messages (sub_input_tokens, sub_output_tokens, sub_cache_creation_input_tokens, sub_cache_read_input_tokens) ; spawns (count of Agent/Task calls) ; spawn_types (map). Only emitted when non-empty.
- isMainEvent(ev) (repl.go:369): true when parent_tool_use_id absent/nil/"".

## metrics.go (62 lines)
- type Metrics {dir, flags, mu}. func newMetrics(dir string, flags map[string]any) *Metrics ; dir from a.MetricsDir (repl.go:1141). CLI default: filepath.Join("notes","metrics") (metricscmd.go:16), relative to cwd. dir=="" disables.
- func (m *Metrics) Log(rec map[string]any) error : appends one JSON line to <dir>/<YYYY-MM-DD>.jsonl (UTC date) ; adds "ts" (RFC3339 UTC) and "flags".
- func usageFields(usage map[string]any) map[string]any : input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, ctx (=ctxTokens).
- ctxTokens (core.go:74) : input_tokens + cache_read_input_tokens + cache_creation_input_tokens of one API call = context-size proxy.
- turn row (repl.go ~415-441): stage="turn", source="background", cost_usd=result.total_cost_usd, cost_turn_usd, result_<usageFields>, result_model_usage (result.modelUsage), plus spend.fields().
- related (core.go): observedTTL reads usage.cache_creation.ephemeral_1h_input_tokens / ephemeral_5m_input_tokens.

## doubts
- whether result.usage is per-turn or session-cumulative not verified in these 4 files (only turnCostUSD explicitly de-cumulates cost).
- fmtRateStatus scale rule (v<1 => ×100) is a heuristic.
