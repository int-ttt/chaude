# Claude Code subagent: context inheritance and model selection

source-primary: https://code.claude.com/docs/en/sub-agents (official, fetched 2026-10-30 via WebFetch, which is a summarizing fetch; quotes are as returned, not raw page bytes)
checked-as-of: 2026-10-30
note: the page is ~116k chars; WebFetch read chars 0-100000 only in pass 1, and pass 2 covered the Fork / Choose a model / one-model sections.

| # | question | answer | evidence (section heading in docs) | source URL | status |
|---|---|---|---|---|---|
| 1 | Does a normal subagent inherit parent conversation context? | No. Starts with a fresh, isolated context window; does not see conversation history, invoked skills, or files already read. Parent writes a delegation (task) message. | "Manage subagent context > What loads at startup": "Each subagent starts with a fresh, isolated context window..." | https://code.claude.com/docs/en/sub-agents | verified (quote via WebFetch) |
| 2 | What does a non-fork subagent start with? | System prompt (agent's own prompt + basic environment details, not Claude Code's system prompt) + task message + CLAUDE.md, git status snapshot, preloaded skills, sibling roster. Output style and auto memory of main conversation do not reach it. | "Manage subagent context > What loads at startup"; "Write subagent files" ("Subagents receive only this system prompt plus basic environment details...") | https://code.claude.com/docs/en/sub-agents | verified (quote via WebFetch) |
| 3 | Exception to no-inheritance? | Fork: "inherits the parent conversation instead of starting fresh." Forks receive the main conversation's exact tool pool. | "Manage subagent context > What loads at startup" (link to #fork-the-current-conversation); "Control subagent capabilities > Available tools" | https://code.claude.com/docs/en/sub-agents | verified in part; fork section body itself NOT read (unverified: how a fork is triggered) |
| 4 | Can the model differ from the parent's? | Yes. Frontmatter `model` accepts `sonnet`, `opus`, `haiku`, `fable`, a full model ID (example in docs: `claude-opus-5-5`), or `inherit`. Omitted = resolved by order below. | "Frontmatter reference" table; "Choose a model" | https://code.claude.com/docs/en/sub-agents | verified (quote via WebFetch) |
| 5 | Model resolution order when not fixed | 1) per-invocation `model` parameter 2) subagent definition `model` frontmatter (`inherit` = main conversation's model) 3) `CLAUDE_CODE_SUBAGENT_MODEL` env var (alias or model ID) 4) main conversation's model. An installed mod's `agent.spawn` hook model replaces the per-invocation parameter. | "Choose a model" | https://code.claude.com/docs/en/sub-agents | verified (quote via WebFetch) |
| 6 | Can the Agent tool call override the model per invocation? | Yes, but only with an alias: `sonnet`, `opus`, `haiku`, `fable`. Full model ID and `inherit` are NOT accepted on the Agent call. Applies also when the subagent is resumed / sent a follow-up. | "Choose a model" | https://code.claude.com/docs/en/sub-agents | verified (quote via WebFetch) |
| 7 | Force one model for all subagents | `CLAUDE_CODE_SUBAGENT_MODEL` is a default only. Adding `CLAUDE_CODE_SUBAGENT_MODEL_FORCE=1` makes every subagent use it: frontmatter `model` ignored and Claude cannot pass a model at spawn. Built-in Explore keeps its own listed model when only FORCE is set (per docs). | "Choose a model > Run every subagent on one model" | https://code.claude.com/docs/en/sub-agents | verified (quote via WebFetch) |
| 8 | Built-in subagent models | general-purpose: `CLAUDE_CODE_SUBAGENT_MODEL` if set, else main conversation model. claude-code-guide: Haiku. statusline-setup: Sonnet. Explore: listed under Built-in subagents (value not captured). | "Choose a model > Built-in subagents" / "Other" table | https://code.claude.com/docs/en/sub-agents | partially verified (Explore value unverified) |
| 9 | Nesting | Third-party guide claims "Subagents cannot spawn other subagents." Not checked against official text. | third-party search snippet only | https://vantaige.io/fr/blog/claude-code-subagents-save-context-3-patterns (secondary, search snippet) | unverified against official docs |

## Conflicts / caveats
- Third-party search snippets disagreed on precedence (one said env var is highest). Official "Choose a model" order (row 5) was used. Env var is lower than per-invocation and frontmatter per docs text.
- Tool name: the question says "Agent 도구"; docs call it the Agent tool on Agent calls (older name Task may appear elsewhere; not checked).
- The doc mentions models `fable` and `claude-opus-5-5`; taken as written in the fetched docs, not independently verified against model list.

## Sources
- https://code.claude.com/docs/en/sub-agents (primary, official)
- https://vantaige.io/fr/blog/claude-code-subagents-save-context-3-patterns (secondary, unverified claim only)
- https://glukhov.org/ai-devtools/claude-code/claude-code-subagents/ (secondary, search result only, not fetched)
- https://tembo.io/blog/claude-code-subagents (secondary, search result only, not fetched)
