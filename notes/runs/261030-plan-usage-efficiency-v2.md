# plan: usage efficiency v2 (headless A/B)
goal: measure, on a fixed input with fixed expected output, (C) split vs batched haiku-scout reads and (P) current vs trimmed opus-architect prompt; then implement only what data supports.
prior: 261030-result-usage-close.md (C n=2: B/A cost 0.588, cacheCreate 0.509, high variance); cold spawn ~20957 cache-write tok; opus-architect first-msg cache write median 7308.
budget: 5h usage start 43%, hard stop 60%; poll `claude -p /usage` at most once per 5 min; do not start a new block if >=57%.
constraints: no edits to CLAUDE.md or .claude/agents/*.md in repo (trim lives only in /tmp copy); no commit/push/PR.

## tool: scripts/ab_headless.py (new, stdlib only, python3)
subcommand run: `python3 scripts/ab_headless.py run --spec <spec.json> --order A,B,B,A,... --out <file.jsonl> [--warmup A,B]`
- spec.json: {"expected": "<python regex, fullmatch on result.strip()>", "arms": {"<label>": {"cwd": "<abs dir>", "model": "haiku", "prompt": "<text>", "extra_args": ["..."]}}}
- per run: subprocess.run(["claude","-p","--model",model,"--output-format","json","--no-session-persistence",*extra_args,prompt], cwd=cwd, capture_output=True, text=True, timeout=900)
- warmup labels run first, rows marked warmup=true.
- append one JSON line per run: ts (UTC ISO), arm, idx, warmup, rc, is_error, total_cost_usd, num_turns, duration_ms, result_sha256 (sha256 of result.strip()), result_head (first 200 chars), correct (bool), modelUsage {model: {inputTokens, outputTokens, cacheReadInputTokens, cacheCreationInputTokens, costUSD}}. On parse failure write rc, is_error=true, stderr_head (500 chars).
subcommand report: `python3 scripts/ab_headless.py report <file.jsonl> --a A --b B`
- excludes warmup rows; per arm: n, correct count, unique result_sha256 count, cost mean/sd/min/max, per model mean cacheCreationInputTokens / cacheReadInputTokens / costUSD.
- ratio b/a of mean cost and of total cacheCreation; exact two-sided Mann-Whitney U p-value on total_cost_usd (enumerate all splits; n<=10 per arm).
- print plain text; also `--json` prints the same as one JSON object.

## arm C: split vs batched scout reads
cwd: /tmp/abv2/C (mkdir; copy repo .claude/agents/ into /tmp/abv2/C/.claude/agents/; no settings.json, no CLAUDE.md)
model haiku; extra_args: permission flags that worked in prior /tmp/abcap runs (runner checks; else `--allowedTools Agent Read Bash Glob Grep`).
expected: `hosts=\d+ shells=\d+` and all correct rows must share one result_sha256.
A prompt: "Use the Agent tool twice with subagent_type haiku-scout, as two separate calls: first to count the lines of /etc/hosts, second to count the lines of /etc/shells (wc -l). Then reply with exactly one line and nothing else: hosts=<n> shells=<m>"
B prompt: "Use the Agent tool once with subagent_type haiku-scout to count the lines of both /etc/hosts and /etc/shells (wc -l) in that single call. Then reply with exactly one line and nothing else: hosts=<n> shells=<m>"
order: warmup A,B then A,B,B,A,A,B,B,A,A,B (N=5 each).

## arm P: opus-architect current vs trimmed
cwd: /tmp/abv2/PA (current copy) and /tmp/abv2/PB (same copy, opus-architect.md body replaced by TRIM below; frontmatter unchanged). Same path length on purpose.
model haiku (main); opus usage in modelUsage = opus-architect only.
expected: `more=/etc/hosts hosts=\d+ shells=\d+|more=/etc/shells hosts=\d+ shells=\d+`
prompt (both): "Use the Agent tool once with subagent_type opus-architect and this exact prompt: 'Decide which file has more lines, /etc/hosts or /etc/shells. Delegate the counting. Do not write any files. Final answer exactly one line: more=<path> hosts=<n> shells=<m>'. Then reply with exactly the subagent's final answer line and nothing else."
order: warmup A,B then A,B,B,A,A,B,B,A,A,B. Poll /usage before warmup and after every 4 runs (respect 5-min limit).
TRIM (body of /tmp/abv2/PB/.claude/agents/opus-architect.md after the closing ---):
Agent tool only (no file access). Delegate: reads -> `haiku-scout` (RETURN: verbatim <=4000 chars); edits/code -> `haiku-coder` (exact spec), then `haiku-checker`; notes/runs/ saves -> `haiku-batch-worker` (verbatim). Briefs: exact paths, line ranges, wording; <=5 lines for mechanical steps. Never delegate to Opus.
Start from the brief (user's words or ESCALATE file); don't redo its reading. Read only decision-relevant files (grep first) plus notes/FACTS.md, DECISIONS.md, LESSONS.md; never repeat LESSONS.md failures.
1. Decide: problem, constraints, 2-3 options, choice and why.
2. Small: haiku-coder implements, haiku-checker runs the narrowest check; report the exact result.
3. Else haiku-batch-worker saves `notes/runs/<YYMMDD>-plan-<topic>.md`: goal, approach and why, ordered per-file steps (names, signatures, formats, wording), edge cases, acceptance checks, must-not-change.
Plan review: scout reads the plan; batch-worker writes concrete improvements to the brief's path (default `<plan>-review.md`); don't execute. Return <=3 lines.
notes/* in terse English `key: value` (Korean values verbatim). User-facing: Korean 존댓말. Final message <=5 lines: outcome · changed paths · checks PASS/FAIL · user-only question; details in notes/runs/. Over ~20 responses: split.

## decision rules (data only)
- adopt B over A if: all B rows correct, single result_sha256 equal to A's, mean cost ratio b/a <= 0.90, and Mann-Whitney p < 0.05 (N=5 v 5 min p = 0.0079).
- if ratio <= 0.90 but p >= 0.05: "indicative", no harness change; report as user decision only.
- if any B row incorrect: reject B regardless of cost.

## outputs
- data: notes/runs/261030-ab-v2-C.jsonl, notes/runs/261030-ab-v2-P.jsonl (copy from /tmp/abv2)
- result: notes/runs/261030-result-usage-efficiency-v2.md (report text per arm, usage % per poll, decision, implemented changes, checks)
## must NOT change
CLAUDE.md, .claude/agents/*.md, .claude/settings.json, hooks; existing Go behavior unless data supports a change; no commit/push.

## review merged (from 261030-plan-usage-efficiency-v2-review.md; overrides above where they differ)
- extra_args of every arm add `--max-budget-usd 1.5`.
- run: TimeoutExpired -> is_error=true, rc=-9, continue.
- new subcommand `selftest`: MW p fully separated 5v5 == 2/252 (abs diff < 1e-9); identical samples == 1.0; exits 0 on pass.
- report also prints mean num_turns and per-model mean costUSD.
- order: arm C complete, then arm P.
- expected regex pinned to ground truth from `wc -l < /etc/hosts` and `wc -l < /etc/shells` (e.g. `hosts=12 shells=11`).
- early stop: clear decision at N=5 -> stop; p in [0.05,0.2) and usage < 52% -> one more ABBA block once.
- record `claude --version` in result file.
