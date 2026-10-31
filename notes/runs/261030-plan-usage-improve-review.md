# 261030-plan-usage-improve-review
target: notes/runs/261030-plan-usage-improve.md (v1)
verdict: approach B holds; 7 improvements, all merged into v2
1. M0 must save its script to /tmp/claude-usage/measure.py and its output to notes/runs/261030-usage-m0.txt so M1 reruns the same script; otherwise deltas are not comparable.
2. Units: keep API-equivalent USD (client estimate, total_cost_usd / price table) separate from server 5h %. Never claim equality; compare trends only.
3. Rate logging must reuse the value already fetched for the status line. No new `claude -p /usage` call or API call just to log (that would itself consume usage).
4. Rate rows: write only when the 5h % value changes or at most once per turn; on any error skip silently; never block the TUI.
5. Measure per-spawn cold start: first assistant message cache_creation per subagent transcript (review risk L15 of 261030-plan-usage-window-review.md), grouped by agent type.
6. Observer effect: this round's own delegations are counted in M1; report them as a separate line, not as background usage.
7. Delegation budget for this round: <=12 Agent calls total; one coder instance only; no Opus/Sonnet except the final sonnet-reviewer.
