# research: claude code budget flag and cost/usage output
date: 2026-10-29
scope: (a) budget flag, (b) -p json/stream-json cost, (c) stream-json usage cache_creation ephemeral fields
status: done (a,b,c via official docs); local `claude --help` check NOT run (no shell tool in this run)

## done / remaining
- done: fetched cli-reference, headless, agent-sdk cost-tracking, agent-sdk typescript (Usage type), platform prompt-caching
- done: extracted --max-budget-usd row (cli-reference, lines 105 of fetched text)
- remaining: local `claude --help` grep for "budget" / "cost" (requires Bash; not available here)
- remaining: verify CLI stream-json sample line directly (needs a real run; not done)

## table
| claim | verdict | source URL |
| :- | :- | :- |
| (a) flag `--max-budget-usd` exists in Claude Code CLI | 확인됨 | https://code.claude.com/docs/en/cli-reference (CLI flags table) |
| (a) exact help text (docs row): "Stop the run once estimated spend on API calls reaches this amount (print mode only). Claude Code checks the cap against its client-side cost estimate, which can differ from your bill. Spend from subagents counts toward the cap. Spend can pass the cap, so leave headroom. When you return to a conversation with --continue or --resume, totals restored from earlier runs don't count toward it. Once spend reaches the cap, spawning another subagent fails with `Budget limit reached`, and Claude Code stops background subagents that are still running; the cap-enforcement behaviors require Claude Code v2.1.217 or later" | 확인됨 (docs text; local --help wording not verified) | https://code.claude.com/docs/en/cli-reference |
| (a) example usage: `claude -p --max-budget-usd 5.00 "query"` | 확인됨 | https://code.claude.com/docs/en/cli-reference |
| (a) cap is print-mode only (`-p`) | 확인됨 | https://code.claude.com/docs/en/cli-reference |
| (a) budget stop error result subtype `error_max_budget_usd`; usage omits the response that crossed the cap, total_cost_usd and modelUsage include it | 확인됨 | https://code.claude.com/docs/en/agent-sdk/cost-tracking |
| (a) on budget cap, remaining background work is stopped instead of waited on | 확인됨 | https://code.claude.com/docs/en/headless |
| (a) subscription (Claude.ai) login: cap behavior vs included usage | 확인 안 됨 (only a search snippet of a closed "not planned" third-party bug report; not fetched) | https://claudeissues.com/issue/85400-bug-max-budget-usd-stops-max-subscription-run-despite-zero-api-cost-and-remainin |
| (a) local `claude --help` lists budget flag | 확인 안 됨 (not executed) | n/a |
| (b) `-p --output-format json` payload includes `total_cost_usd` | 확인됨 | https://code.claude.com/docs/en/headless (section "Pipe data through Claude") |
| (b) json payload includes per-model cost breakdown | 부분 (docs say "per-model cost breakdown"; JSON field name not shown) | https://code.claude.com/docs/en/headless |
| (b) stream-json: final `result` message carries "final response text, cost, and session metadata" | 확인됨 (wording "cost"; `total_cost_usd` key not shown in stream-json sample) | https://code.claude.com/docs/en/headless (section "Stream responses") |
| (b) `total_cost_usd` is cumulative for the whole `query()` call (SDK), includes subagent requests | 확인됨 | https://code.claude.com/docs/en/agent-sdk/cost-tracking |
| (b) `total_cost_usd` is per-turn | 확인 안 됨 (docs: cumulative per call; per-turn result only in streaming input mode, where totals are still running totals) | https://code.claude.com/docs/en/agent-sdk/cost-tracking |
| (b) `total_cost_usd` is an estimate, not billing | 확인됨 | https://code.claude.com/docs/en/agent-sdk/cost-tracking |
| (b) resumed session: result includes earlier spend of the session (v2.1.277+) | 확인됨 | https://code.claude.com/docs/en/agent-sdk/cost-tracking |
| (c) API usage object: `cache_creation` with `ephemeral_5m_input_tokens` and `ephemeral_1h_input_tokens` | 확인됨 | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| (c) `cache_creation_input_tokens` equals sum of the two ephemeral fields | 확인됨 | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| (c) Agent SDK TS `Usage` type (BetaUsage) includes `cache_creation: { ephemeral_5m_input_tokens, ephemeral_1h_input_tokens } \| null` | 확인됨 | https://code.claude.com/docs/en/agent-sdk/typescript |
| (c) Claude Code JSONL usage contains the cache_creation ephemeral breakdown | 부분 (third-party issue quoting JSONL; no CLI version stated) | https://github.com/ryoppippi/ccusage/issues/899 |
| (c) `claude -p --output-format stream-json` lines include `usage.cache_creation` ephemeral fields | 부분 (SDK type documents it; no official stream-json sample checked; not run locally) | https://code.claude.com/docs/en/agent-sdk/typescript |

## key facts (quoted)
- headless: "With `--output-format json`, the response payload includes `total_cost_usd` and a per-model cost breakdown, so scripted callers can track spend without consulting the usage dashboard."
- cost-tracking WARNING: "The `total_cost_usd` and `costUSD` fields are client-side estimates, not authoritative billing data."
- cost-tracking: "`usage` ... excludes subagent" ; "`total_cost_usd` | Included. Counts subagent requests alongside the top-level loop".
- prompt-caching: "the current `cache_creation_input_tokens` field equals the sum of the values in the `cache_creation` object."

## doubts
- docs (code.claude.com) describe the flag; local `claude --help` wording not confirmed. CLI-reference itself says `claude --help` does not list every flag.
- stream-json field-level sample (total_cost_usd and cache_creation inside stream-json lines) not captured; needs a real `claude -p ... --output-format stream-json --verbose` run.
- subscription-auth budget behavior conflicting info (third-party only).
