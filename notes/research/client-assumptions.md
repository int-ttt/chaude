# client assumptions (sourced table)

access_date: 2026-10-29
full evidence and quotes: /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261029-research-client-assumptions.md

| claim | verdict | source URL | date |
| :- | :- | :- | :- |
| 1a. Default prompt cache TTL is 5 minutes | 확인됨 | https://platform.claude.com/docs/en/build-with-claude/prompt-caching | accessed 2026-10-29, page date not shown |
| 1b. A 1-hour TTL option exists (`"ttl": "1h"`) | 확인됨 | https://platform.claude.com/docs/en/build-with-claude/prompt-caching | accessed 2026-10-29, page date not shown |
| 1c. Write multipliers 1.25x (5m) and 2x (1h); read 0.1x | 확인됨 (read multiplier has per-model footnote exceptions) | https://platform.claude.com/docs/en/build-with-claude/prompt-caching | accessed 2026-10-29, page date not shown |
| 2a. Several Claude Code sessions can run at once in separate terminals, each with a different model | 확인됨 | https://code.claude.com/docs/en/model-config | accessed 2026-10-29, page date not shown |
| 2b. `--model` sets the model for that session only | 확인됨 | https://code.claude.com/docs/en/model-config | accessed 2026-10-29, page date not shown |
| 2c. `/model` switches a model per session | 부분 (interactive `/model` is saved as default; per-session via picker `s` or in `-p`) | https://code.claude.com/docs/en/model-config | accessed 2026-10-29, page date not shown |
| 2d. Concurrent sessions do NOT share a prompt cache | 확인 안 됨 (no official statement; docs say same-workspace identical prefixes can hit) | https://platform.claude.com/docs/en/build-with-claude/prompt-caching | accessed 2026-10-29, page date not shown |
| 3a. Non-interactive start with a given model (`claude -p --model <x>`) | 부분 (`-p` and `--model` documented separately; combined example not written out) | https://code.claude.com/docs/en/cli-reference ; https://code.claude.com/docs/en/headless | accessed 2026-10-29, page date not shown |
| 3b. What `-p` prints | 부분 (text / json / stream-json documented; model field in json not documented) | https://code.claude.com/docs/en/headless ; https://code.claude.com/docs/en/cli-reference | accessed 2026-10-29, page date not shown |
