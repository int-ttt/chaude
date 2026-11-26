# Prompt caching pricing + Claude Code usage/TTL (official sources)
scope: Anthropic official docs only. seen: 2026-11-15 (WebFetch of live pages).

## 1. Cache multipliers (x base input)
| item | value | source | seen |
| :- | :- | :- | :- |
| 5m cache write | 1.25x base input | https://platform.claude.com/docs/en/about-claude/pricing#prompt-caching ; https://platform.claude.com/docs/en/build-with-claude/prompt-caching | 2026-11-15 |
| 1h cache write | 2x base input | same | 2026-11-15 |
| cache read, most models | 0.1x base input | same (pricing footnote: "All other models use the standard 0.1x multiplier.") | 2026-11-15 |
| cache read, Opus 5.5 + Sonnet 5.5 | 0.05x | pricing table footnote 2 | 2026-11-15 |
| cache read, Fable 5.2/Mythos 5.2/Fable 5.1/Mythos 5.1 | 0.025x | pricing table footnote 1 | 2026-11-15 |
| cache read, Haiku 5.5 | 0.1x (standard) | pricing table; 0.01 = 0.1 x 0.10 | 2026-11-15 |
| break-even | 5m: after 1 read; 1h: after 2 reads | prompt-caching doc | 2026-11-15 |

## 2. Prices USD/MTok (base / 5m write / 1h write / cache hit / output)
| model | base | 5m | 1h | hit | out | source | seen |
| :- | :- | :- | :- | :- | :- | :- | :- |
| Haiku 5.5, prompt <=100,000 tok | 0.10 | 0.125 | 0.20 | 0.01 | 0.50 | https://platform.claude.com/docs/en/about-claude/pricing#model-pricing | 2026-11-15 |
| Haiku 5.5, prompt >100,000 tok | 0.50 | 0.625 | 1 | 0.05 | 2.50 | same | 2026-11-15 |
| Opus 5.5 | 4 | 5 | 8 | 0.20 | 20 | same | 2026-11-15 |
| Sonnet 5.5 (ref) | 2 | 2.50 | 4 | 0.10 | 10 | same | 2026-11-15 |

Haiku 5.5 long-context tier:
- threshold: prompt over 100,000 tokens -> higher prices (Haiku 5.5 only; other 4.6+ models have full 1M at standard price).
- counts: "A request's prompt length counts all of its input tokens, including cache reads and cache writes." Per request; a request over threshold pays higher price even if part is cache hit.
- source: https://platform.claude.com/docs/en/about-claude/pricing#long-context-pricing ; seen 2026-11-15
- 1.1x data-residency multiplier (inference_geo=us) also applies to the >100k Haiku price. Source: same page, "Data residency pricing".

## 3. Pro/Max usage limits vs cache reads (Claude Code)
- 5h session + weekly limit; shared across Claude and Claude Code; window starts at first message. Source: https://support.claude.com/en/articles/11145838-use-claude-code-with-your-pro-or-max-plan ; https://support.claude.com/en/articles/11647753-how-do-usage-and-length-limits-work ; seen 2026-11-15
- cache reads: NOT DOCUMENTED (no weighting) on either support page. cache writes, regular input, output weighting, model weighting: NOT DOCUMENTED.
- indirect: https://code.claude.com/docs/en/costs (seen 2026-11-15): "Claude Code re-reads that history at the cached token rate, so a one-line question in a session that has been open all day still draws usage for the whole conversation." -> implies cache reads draw usage; no multiplier stated.

## 4. Claude Code cache TTL default
source: https://code.claude.com/docs/en/prompt-caching ("Which TTL each request gets", "Choose the TTL yourself"); seen 2026-11-15
| bucket | Claude subscription, within plan usage | usage credits / API key / cloud |
| :- | :- | :- |
| main conversation | 1h | 5m |
| subagents + other (workflows, teammates, forks, compaction, titles) | 5m (except server-controlled helper requests: 1h) | 5m |
- override order: FORCE_PROMPT_CACHING_5M=1 > bucket env/setting (CLAUDE_CODE_PROMPT_CACHE_TTL / promptCacheTtl for main; CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL / subagentPromptCacheTtl for others) > subagent frontmatter cacheTtl (v2.1.248+) > ENABLE_PROMPT_CACHING_1H=1 (both buckets) > default. Settings/env need v2.1.242+.
- over plan limit on usage credits: main drops to 5m.
- /usage shows "Prompt cache (main)" line; main only, not subagents. source: https://code.claude.com/docs/en/costs

## 5. FACTS.md check (notes/FACTS.md lines 7-10, checked 2026-10-29)
- Opus 5.5 4/5/8/0.20/20: MATCH.
- Sonnet 5.5 2/2.50/4/0.10/10: MATCH.
- Haiku 5.5 <=100k 0.10/0.125/0.20/0.01/0.50: MATCH; >100k 0.50/0.625/1/0.05/2.50: MATCH.
- FACTS line 6 cites docs.claude.com; it redirects to platform.claude.com (same content). Not a value mismatch.
- FACTS does not record: cache-hit multiplier per model (0.05x / 0.025x exceptions), long-context threshold counts cache reads/writes, Claude Code TTL defaults. Add if needed.

## unverified / not documented
- Pro/Max cache-read weighting: not documented.
- Pricing page has no publish date; "seen" = fetch date.
- Search snippets claimed Opus 5.5 cache read 0.1x and Sonnet 5.5 0.05x conflict; resolved by official footnotes above (Opus 5.5 0.05x, Sonnet 5.5 0.05x). Third-party posts ignored.
