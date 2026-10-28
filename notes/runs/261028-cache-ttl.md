# Prompt cache TTL: official sources (fetched 2026-10-28)

task: find official prompt cache TTL (API docs + Claude Code docs)
status: done
docs_claude_com: redirects to https://platform.claude.com/docs/en/build-with-claude/prompt-caching (302)

## Table

| # | Question | Answer | Quote (verbatim) | Source URL |
|---|---|---|---|---|
| 1 | Default TTL | 5 minutes (API default) | "By default, the cache has a 5-minute lifetime. The cache is refreshed for no additional cost each time the cached content is used." (section "How prompt caching works") | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 2a | 1h TTL option | Yes, via `ttl: "1h"` in `cache_control` | "To use the extended cache, include `ttl` in the `cache_control` definition like this: ... \"ttl\": \"1h\"" (section "1-hour cache duration") | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 2b | 1h pricing | Cache write 2x base input | "Cache write tokens with a 1-hour TTL are 2 times the base input tokens price." (section "Pricing") | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 2c | 5m write pricing | Cache write 1.25x base input | "Cache write tokens with a 5-minute TTL are 1.25 times the base input tokens price." (section "Pricing") | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 2d | Cache read pricing | 0.1x base input (0.025x Fable 5.1 / Mythos 5.1; 0.05x Opus 5.5 / Sonnet 5.5) | "Cache read tokens are 0.1 times the base input tokens price (see the table footnote for per-model exceptions)." | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 3a | Cache hit refreshes TTL (API) | Yes, refresh at no extra cost | "The cache is refreshed for no additional cost each time the cached content is used." | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 3b | Cache hit refreshes TTL (Claude Code) | Yes, resets timer | "Each request that hits the cache resets the timer, so the cache stays warm as long as you keep working." (section "Cache lifetime") | https://code.claude.com/docs/en/prompt-caching.md |
| 4a | Claude Code TTL, subscription within plan usage | Main conversation 1h; other requests 5m (except server-controlled helpers 1h) | "Unless you choose a TTL yourself, Claude Code requests the one-hour TTL only on a Claude subscription within your plan's included usage." (section "Which TTL each request gets"; table) | https://code.claude.com/docs/en/prompt-caching.md |
| 4b | Claude Code TTL, API key / cloud provider | Main 5m; others 5m | Table row "Usage credits, API key, or cloud provider": Main conversation "Five minutes"; Everything else "Five minutes" | https://code.claude.com/docs/en/prompt-caching.md |
| 4c | Overage (usage credits) | Main conversation drops to 5m unless user sets TTL | "Once you go over your plan's usage limit and Claude Code draws on usage credits, you are billed for that usage, so Claude Code drops the main conversation to the five-minute TTL, which charges a lower rate for cache writes. To keep the one-hour TTL there, choose the TTL yourself." | https://code.claude.com/docs/en/prompt-caching.md |
| 5a | Setting: main conversation | `promptCacheTtl` setting or `CLAUDE_CODE_PROMPT_CACHE_TTL` env; values `5m` or `1h` | "Main conversation: the promptCacheTtl setting, or the CLAUDE_CODE_PROMPT_CACHE_TTL environment variable" ; "Each control takes 5m or 1h, and Claude Code ignores any other value." | https://code.claude.com/docs/en/prompt-caching.md |
| 5b | Setting: subagents | `subagentPromptCacheTtl` or `CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL` | "Everything else: the subagentPromptCacheTtl setting, or the CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL environment variable" | https://code.claude.com/docs/en/prompt-caching.md |
| 5c | Version req | v2.1.242+ for the four TTL controls above | "Both settings and both environment variables require Claude Code v2.1.242 or later." | https://code.claude.com/docs/en/prompt-caching.md |
| 5d | Global 1h env | `ENABLE_PROMPT_CACHING_1H=1` requests 1h for both buckets (precedence 5 of 6) | Precedence list item 5: "ENABLE_PROMPT_CACHING_1H=1, which requests one hour for both buckets" | https://code.claude.com/docs/en/prompt-caching.md |
| 5e | Force 5m env | `FORCE_PROMPT_CACHING_5M=1` forces 5m for both buckets (precedence 1, highest) | "FORCE_PROMPT_CACHING_5M=1, which forces five minutes for both buckets" | https://code.claude.com/docs/en/prompt-caching.md |
| 5f | Precedence | FORCE_5M > bucket env > bucket setting > subagent frontmatter `cacheTtl` (v2.1.248+) > ENABLE_PROMPT_CACHING_1H=1 > bucket default | "When more than one control applies, Claude Code takes the first match in this order:" (list of 6) | https://code.claude.com/docs/en/prompt-caching.md |
| 5g | Verify TTL | Check `usage.cache_creation` in `claude -p "hello" --output-format json` (`ephemeral_1h_input_tokens` / `ephemeral_5m_input_tokens`) | "Claude Code reports one-hour cache writes under ephemeral_1h_input_tokens and five-minute cache writes under ephemeral_5m_input_tokens." | https://code.claude.com/docs/en/prompt-caching.md |
| 5h | Disable caching | `DISABLE_PROMPT_CACHING=1` (plus per-model variants) | "To turn it off, set one of these environment variables to 1" | https://code.claude.com/docs/en/prompt-caching.md |
| 5i | Org policy | TTL vars can go in managed settings `env` block | "put any of these or the TTL variables in the env block of managed settings" | https://code.claude.com/docs/en/prompt-caching.md |
| 6 | Session display | `/usage` shows warm/cold + TTL | Example line: "warm (1h TTL, last activity 40s ago)" (requires v2.1.251+) | https://code.claude.com/docs/en/costs.md |

## Notes
- API docs page lists no TTL env vars or settings; those are Claude Code only (row 5a-5i).
- Claude Code 1h default for subscription: only "within your plan's included usage" (row 4a).

## Unverified / doubts
- ENABLE_PROMPT_CACHING_1H: official doc now lists it (5d). Third-party claims (v2.1.108, Bedrock-only deprecated var ENABLE_PROMPT_CACHING_1H_BEDROCK) NOT verified on official pages.
- Community GitHub issue (claudeissues.com #49139, "ENABLE_PROMPT_CACHING_1H is not working (API key user)") and a May 2026 issue saying 1h env always wrote 5m: NOT verified; official doc does not mention a bug. Test with `usage.cache_creation` (5g).
- API doc: refresh rule stated generally; doc does not say explicitly that a 1h entry's refresh restores full 1h. Claude Code doc says "resets the timer" without a number.
- Cached-read/refresh multiplier for Fable 5.1 / Mythos 5.1 / Opus 5.5 / Sonnet 5.5 from footnote (not in Claude Code scope).
- Fetch truncated at 100000 chars of 162264 for platform doc; pricing section was read via a second slice (offset 0 covering pricing text). Table rows not checked against full remaining text.
- Subscription plan names (Pro/Max/Team/Enterprise) not separately verified for the 1h default; doc says "a Claude subscription".
