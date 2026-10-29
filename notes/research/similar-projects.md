# Similar projects survey (2026-10-29)
source: web survey by a research subagent; stars from GitHub API that day; savings self-reported unless marked independent.
benchmark (independent): https://computingforgeeks.com/reduce-claude-code-token-usage-tools/ (Claude Code 2.1.116, one task, baseline 284K tokens)

| project | stars | core mechanism | note for chaude |
|---|---|---|---|
| RTK https://github.com/rtk-ai/rtk | 84.9k | PreToolUse Bash hook rewrites `git status`/tests/ls/grep to `rtk ...`, returns filtered output | only Bash; 0% on short tasks; engine BUSL-1.1; optional add-on, not adopted |
| caveman https://github.com/JuliusBrussee/caveman | 115.8k | terse output style; `/caveman-compress CLAUDE.md` (23-49%/file); independent: JetBrains -8.5% output, no quality loss | main thread must stay readable; agent replies already ≤5 lines. CLAUDE.md compression is the lever for TingTingTengTeng (212k-token CLAUDE.md) |
| claude-mem https://github.com/thedotmack/claude-mem | 104k | hooks → SQLite observations; progressive disclosure (index first, fetch by id) | chaude snapshot already bounded (OPEN 20 lines, LEDGER 5, prev session 6 msgs) |
| context-mode https://github.com/mksglu/context-mode | 32.7k | run tools in sandbox, return only matching chunks of big outputs | PostToolUse cannot replace tool output in Claude Code; not adopted |
| oh-my-claudecode https://github.com/Yeachan-Heo/oh-my-claudecode | 40.4k | model×agent tiers, heuristic router with shadow mode | adopted the deterministic part: explicit `haiku로/opus로/fable로` skips the conductor call |
| claude-code-router https://github.com/musistudio/claude-code-router | 37.9k | proxy routing by request class (background/think/longContext) | API-key proxy; chaude runs on the subscription, not adopted |
| token-savior https://github.com/Mibayy/token-savior | 1.2k | Bash rewriter, output compactors, symbol index, capture sandbox | 106-tool manifest can erase gains; not adopted |
| ccusage https://github.com/ryoppippi/ccusage | 19.2k | usage reports from ~/.claude/projects/*.jsonl | measurement tool; `chaude usage-report` covers the same |
| model-router hooks (tzachbon, 0dust/ClaudeRouter) | <200 | heuristic pre-classifier, escalate on retry | idea adopted (forced model short-circuit); escalation already in policy (corrections → opus) |

adopted: forced-model short-circuit in the conductor (internal/harness/classifier.go forcedModel).
candidates, not adopted (quality risk or needs more design): outline-first Read for big files (deny + symbol outline), Bash output spill-to-file with grep hint, soft "context high" directive before rotation.
