task: claude code billing official-source check
date: 2026-10-29
status: done
outputs:
  - /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261029-research-billing.md
  - /Users/leekwangseok/PycharmProjects/claude-harness/notes/research/billing.md
verdicts:
  claim1 plan-included: 확인됨
  claim2 api-key-separate-billing: 확인됨
  claim3 precedence: 확인됨
  claim3 claude-p-subscription: 부분
  claim4 total_cost_usd-estimate: 부분
  claim5 5h-concurrent-sessions: 확인 안 됨
sources_fetched:
  - https://code.claude.com/docs/en/authentication (earlier session)
  - https://code.claude.com/docs/en/costs (earlier session)
  - https://code.claude.com/docs/en/headless (earlier session)
  - https://support.claude.com/en/articles/11145838-using-claude-code-with-your-pro-or-max-plan (earlier session, re-checked)
  - https://code.claude.com/docs/en/agent-sdk/cost-tracking (this session)
searches:
  - WebSearch: 5-hour session concurrent sessions (third-party only, not used as verdict basis)
not_verified:
  - claude -p subscription coverage explicit sentence
  - concurrent session shared 5h limit (official)
  - https://support.claude.com/en/articles/12429409-extra-usage-for-paid-claude-plans (link only, not fetched)
  - page dates (none shown)
extended_search: not run (standard sufficient for verdicts; gaps marked 확인 안 됨)
reply_constraint_note: system format (<=3 lines, <=300 chars) applied; per-claim verdicts and URLs are in the table file
