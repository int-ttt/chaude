# research: client assumptions (sourced table)

access_date: 2026-10-29
method: WebSearch (standard) then WebFetch on official pages (platform.claude.com, code.claude.com). Quotes are WebFetch extractions of the official page text, not byte-verified copies.
pages_have_visible_date: no (none of the three pages shows a published or updated date; date column = access date)

## Table

| claim | verdict | source URL | date |
| :- | :- | :- | :- |
| 1a. Default prompt cache TTL is 5 minutes | 확인됨 | https://platform.claude.com/docs/en/build-with-claude/prompt-caching | accessed 2026-10-29, page date not shown |
| 1b. A 1-hour TTL option exists (`"ttl": "1h"`) | 확인됨 | https://platform.claude.com/docs/en/build-with-claude/prompt-caching | accessed 2026-10-29, page date not shown |
| 1c. Write multipliers 1.25x (5m) and 2x (1h); read 0.1x | 확인됨 (read multiplier has per-model footnote exceptions) | https://platform.claude.com/docs/en/build-with-claude/prompt-caching | accessed 2026-10-29, page date not shown |
| 2a. Several Claude Code sessions can run at once in separate terminals, each with a different model | 확인됨 | https://code.claude.com/docs/en/model-config | accessed 2026-10-29, page date not shown |
| 2b. `--model` sets the model for that session only (not shared with other sessions) | 확인됨 | https://code.claude.com/docs/en/model-config | accessed 2026-10-29, page date not shown |
| 2c. `/model` switches a model per session (the claim's implied mechanism) | 부분 (interactive `/model` is saved as the default for new sessions; per-session only via picker `s` or in `-p` mode) | https://code.claude.com/docs/en/model-config | accessed 2026-10-29, page date not shown |
| 2d. Concurrent sessions do NOT share a prompt cache | 확인 안 됨 (no official statement either way; see note) | https://platform.claude.com/docs/en/build-with-claude/prompt-caching | accessed 2026-10-29, page date not shown |
| 3a. Non-interactive start with a given model exists (`claude -p --model <x>`) | 부분 (`-p` and `--model` are both documented; the combined example is not written out in the docs) | https://code.claude.com/docs/en/cli-reference ; https://code.claude.com/docs/en/headless | accessed 2026-10-29, page date not shown |
| 3b. What `-p` prints | 부분 (text / json / stream-json documented; json model field not documented) | https://code.claude.com/docs/en/headless ; https://code.claude.com/docs/en/cli-reference | accessed 2026-10-29, page date not shown |

## Evidence (verbatim where quoted)

1a. https://platform.claude.com/docs/en/build-with-claude/prompt-caching (section: intro and "Automatic caching")
- "By default, the cache has a 5-minute lifetime. The cache is refreshed for no additional cost each time the cached content is used."
- "By default, automatic caching uses a 5-minute TTL. You can specify a 1-hour TTL at 2x the base input token price:"

1b. same URL
- "If you find that 5 minutes is too short, Anthropic also offers a 1-hour cache duration [at additional cost]."
- `"cache_control": { "type": "ephemeral", "ttl": "1h" }`

1c. same URL (pricing section Note and table footnotes)
- "Cache write tokens with a 5-minute TTL are 1.25 times the base input tokens price."
- "Cache write tokens with a 1-hour TTL are 2 times the base input tokens price."
- "Cache read tokens are 0.1 times the base input tokens price (see the table footnote for per-model exceptions)."
- Footnotes: "Cache hits and refreshes on Claude Fable 5.1 and Claude Mythos 5.1 are priced at 0.025x the base input price." / "Cache hits and refreshes on Claude Opus 5.5 and Claude Sonnet 5.5 are priced at 0.05x the base input price." / "All other models use the standard 0.1x multiplier."

2a/2b/2c. https://code.claude.com/docs/en/model-config (section: Setting your model)
- "1. During session: use `/model <alias|name>` to switch immediately, or run `/model` with no argument to open the picker."
- "`/model` saves your choice as the default for new sessions by writing the `model` field in your user settings."
- Picker: "`s`: switch model for this session only and leave your default unchanged."
- "The `--model` flag and `ANTHROPIC_MODEL` environment variable apply only to the session you launch with them. To run different models in different terminals at the same time, launch each one with its own `--model` flag rather than switching with `/model`."
- "If you set a model with `/model` in non-interactive mode, with the `-p` flag, your choice applies to the current session only and isn't saved as your default; `/model` in that mode requires Claude Code v2.1.205 or later."
- Precedence (highest first): `/model`; `claude --model`; `ANTHROPIC_MODEL`; settings `model` field; `ANTHROPIC_DEFAULT_MODEL`.

2d. https://platform.claude.com/docs/en/build-with-claude/prompt-caching (section: "Cache storage and sharing")
- "Caches are isolated between organizations. Different organizations never share caches, even if they use identical prompts."
- "Caches are also isolated by workspace within an organization on the Claude API, Claude Platform on AWS, and Microsoft Foundry; Bedrock and Claude Models on Google Cloud (Google managed) use organization-level isolation only."
- "Exact matching: Cache hits require 100 percent identical prompt segments, including all text and images up to and including the block marked with cache control."
- Not stated: isolation per session, per process, per model, or per API key. The "What invalidates the cache" table does not list a model change.
- Inference (not an official statement): two sessions in the same workspace with byte-identical prefixes could hit the same cache entry. So "they should not share" is not supported by the docs and is labeled 확인 안 됨.

3a. https://code.claude.com/docs/en/cli-reference (CLI flags table)
- `--model`: "Sets the model for the current session with a model alias such as `sonnet`, `opus`, `haiku`, or `fable`, or a model's full name. Overrides the `model` setting and `ANTHROPIC_MODEL`."
- `--print`, `-p`: "Print response without interactive mode".
- Example in CLI reference: `claude --model claude-sonnet-5`.
- https://code.claude.com/docs/en/headless: "To run Claude Code in non-interactive mode, pass `-p` with your prompt and the CLI options you need". Rejected with `-p`: `--bg`, and `--cloud` with a task description. `--model` is not listed as rejected.

3b. https://code.claude.com/docs/en/headless (section: Get structured output)
- `text` (default): "plain text output"
- `json`: "structured JSON with result, session ID, and metadata"
- `stream-json`: "newline-delimited JSON for real-time streaming"
- "With `--output-format json`, the response payload includes `total_cost_usd` and a per-model cost breakdown"
- Exit codes: "Claude Code exits with code 0 on success and a non-zero code when the run fails"
- stream-json only: "The `system/init` event reports session metadata including the model, tools, MCP servers, and loaded plugins."
- Not found: a documented `model` field inside the `json` (non-stream) output. Marked 확인 안 됨 for that detail.

## Unverified / not used

- GitHub issue https://github.com/anthropics/claude-code/issues/27627 reports that `/model` in one terminal also switched other open terminals. Seen only as a search snippet; not fetched; current status unknown. 확인 안 됨.
- Third-party blog claims Anthropic cut the default TTL from 1 hour to 5 minutes around 2026-03-06/07. Single unofficial source; not used. The official page states the 5-minute default and the 1-hour option.
- Third-party Requesty page says `/model` may be ignored and recommends `ANTHROPIC_MODEL`. Not verified; official page describes `/model` as working.

## Run log

- done: search claim 1 (standard) -> official page fetched
- done: search claim 2 and 3 (standard) -> official model-config, headless, cli-reference fetched
- done: pricing footnotes and cache-isolation sections checked
- remaining: none (extended search not needed; all three claims have official-source answers or explicit 확인 안 됨)
