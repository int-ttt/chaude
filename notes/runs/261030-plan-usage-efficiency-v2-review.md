# review: 261030-plan-usage-efficiency-v2.md
verdict: sound; 8 improvements, merged into plan section "## review merged".
R1 runaway guard: add `--max-budget-usd 1.5` to every arm's extra_args (opus arm could loop).
R2 timeout: catch subprocess.TimeoutExpired -> row is_error=true, rc=-9, continue.
R3 self-test: add subcommand `selftest` asserting exact MW p for fully separated 5v5 == 2/252 (0.0079365) and for identical samples == 1.0 (ties: midranks, p = share of splits with |U-mean| >= observed).
R4 order: run arm C fully before arm P (C is cheap, answers batching rule first; P is the expensive one).
R5 isolation: both arms share user-level settings/CLAUDE.md equally; record `claude --version` once in result file. Do not add --setting-sources (would differ from real use).
R6 behavior metric: report must also print mean num_turns and per-model costUSD so a trim that changes delegation count is visible, not only total cost.
R7 stop early: if after N=5 the decision rule gives a clear adopt/reject, do not add runs; if p in [0.05,0.2) and usage < 52%, add one more ABBA block (N=7 each) once.
R8 expected-value pinning: runner records `wc -l < /etc/hosts` and `wc -l < /etc/shells` before runs; spec expected regex must use those exact numbers (not \d+) so correctness is checked against ground truth.
