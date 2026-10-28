# prompt cache TTL (copy of notes/runs/261028-cache-ttl.md; fetched 2026-10-28)

| Question | Answer | Quote (verbatim) | Source URL |
|---|---|---|---|
| Default TTL | 5 min | "By default, the cache has a 5-minute lifetime. The cache is refreshed for no additional cost each time the cached content is used." | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 1h option | `cache_control` `ttl: "1h"` | "To use the extended cache, include `ttl` in the `cache_control` definition like this: ... \"ttl\": \"1h\"" | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 1h price | cache write 2x base input | "Cache write tokens with a 1-hour TTL are 2 times the base input tokens price." | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| 5m write price | 1.25x base input | "Cache write tokens with a 5-minute TTL are 1.25 times the base input tokens price." | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| Cache read price | 0.1x base input | "Cache read tokens are 0.1 times the base input tokens price (see the table footnote for per-model exceptions)." | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| Hit refreshes TTL (API) | yes | "The cache is refreshed for no additional cost each time the cached content is used." | https://platform.claude.com/docs/en/build-with-claude/prompt-caching |
| Hit refreshes TTL (Claude Code) | yes | "Each request that hits the cache resets the timer, so the cache stays warm as long as you keep working." | https://code.claude.com/docs/en/prompt-caching.md |
| Claude Code, subscription within plan | main 1h; others 5m (helpers 1h) | "Unless you choose a TTL yourself, Claude Code requests the one-hour TTL only on a Claude subscription within your plan's included usage." | https://code.claude.com/docs/en/prompt-caching.md |
| Claude Code, API key / cloud | main 5m; others 5m | Table row "Usage credits, API key, or cloud provider": "Five minutes" | https://code.claude.com/docs/en/prompt-caching.md |
| Overage | main drops to 5m | "Once you go over your plan's usage limit and Claude Code draws on usage credits, ... Claude Code drops the main conversation to the five-minute TTL" | https://code.claude.com/docs/en/prompt-caching.md |
| Setting, main | `promptCacheTtl` / `CLAUDE_CODE_PROMPT_CACHE_TTL` (`5m` or `1h`) | "Each control takes 5m or 1h, and Claude Code ignores any other value." | https://code.claude.com/docs/en/prompt-caching.md |
| Setting, subagents | `subagentPromptCacheTtl` / `CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL` | "Everything else: the subagentPromptCacheTtl setting, or the CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL environment variable" | https://code.claude.com/docs/en/prompt-caching.md |
| Version | v2.1.242+ | "Both settings and both environment variables require Claude Code v2.1.242 or later." | https://code.claude.com/docs/en/prompt-caching.md |
| Global 1h env | `ENABLE_PROMPT_CACHING_1H=1` | "ENABLE_PROMPT_CACHING_1H=1, which requests one hour for both buckets" | https://code.claude.com/docs/en/prompt-caching.md |
| Force 5m env | `FORCE_PROMPT_CACHING_5M=1` | "FORCE_PROMPT_CACHING_5M=1, which forces five minutes for both buckets" | https://code.claude.com/docs/en/prompt-caching.md |
| Precedence | FORCE_5M > bucket env > bucket setting > subagent `cacheTtl` > ENABLE_1H > default | "When more than one control applies, Claude Code takes the first match in this order:" | https://code.claude.com/docs/en/prompt-caching.md |
| Verify | `usage.cache_creation` | "Claude Code reports one-hour cache writes under ephemeral_1h_input_tokens and five-minute cache writes under ephemeral_5m_input_tokens." | https://code.claude.com/docs/en/prompt-caching.md |

Unverified: third-party claims about ENABLE_PROMPT_CACHING_1H (v2.1.108, Bedrock-only deprecated var) and a community bug report (claudeissues.com #49139) are not confirmed by official pages.
