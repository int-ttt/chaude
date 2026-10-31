task: prompt cache TTL, pricing, /clear cache behavior (official docs)
agent: haiku-researcher
asof: 2026-10-30
output: notes/research/prompt-cache-ttl-clear.md
sources:
  - https://platform.claude.com/docs/en/build-with-claude/prompt-caching (re-checked 2 fetches)
  - https://code.claude.com/docs/en/prompt-caching.md (re-checked)
  - https://code.claude.com/docs/en/commands (/clear row, memory sentence, saved raw output)
  - https://code.claude.com/docs/en/costs (/clear costs nothing)
done: API page 0-100000 chars; Claude Code cache page; commands page; costs page
remaining: API page chars 100000-165860 (not read)
key findings:
  - default TTL: 5m
  - 1h TTL: opt-in, write 2x; 5m write 1.25x; read 0.1x (0.05x Opus/Sonnet 5.5, 0.025x Fable/Mythos 5.1)
  - refresh on hit: yes, no extra cost
  - lookback: 20 blocks per breakpoint
  - /clear server-cache deletion: not stated in any page read
  - /clear and cache hit for identical prefix: API rule yes; Claude Code /clear-specific not stated
doubts:
  - API page tail unread; manual deletion absence unconfirmed beyond 100000 chars
  - costs page quote came from summarizer output
status: DONE, verbatim return to caller
