---
task: REPL(pty) vs claude -p cumulative tokens, opus (claude-opus-5-5), 4 turns, claude 2.1.314
decision: option (a) - pty child env pops denylist only: ANTHROPIC_API_KEY, CLAUDECODE, CLAUDE_CODE_CHILD_SESSION, CLAUDE_CODE_SESSION_ID, CLAUDE_CODE_SESSION_ATTENDED, CLAUDE_CODE_ENTRYPOINT, CLAUDE_CODE_MESSAGING_SOCKET, CLAUDE_CODE_MESSAGING_TOKEN, CLAUDE_PID. config vars (PROMPT_CACHE_TTL, AUTO_COMPACT_WINDOW, EFFORT...) kept to match print run.
why: (b) needs user action outside session; (c) screen scrape weaker than JSONL; (a) = /tmp script edit only, no product code.
script: /private/tmp/replcost.WorkyL/run_compare.py (child_env lines 73-80, --only {print,repl,both})
repl_jsonl: /Users/leekwangseok/.claude/projects/-private-tmp-replcost-WorkyL-work-repl/562a8aa2-1afd-487b-a604-a141880c356f.jsonl
repl_result: /private/tmp/replcost.WorkyL/result_repl.md ; print_result: /private/tmp/replcost.WorkyL/result.md
repl per turn (input/output/cache_read/cache_creation/cum_total/context): 1: 2/67/0/41499/41568/41501 ; 2: 4/3/41499/93/83167/41596 ; 3: 4/3/41597/24/124795/41625 ; 4: 4/28/41626/28/166481/41658 ; sum 14/101/124722/41644/166481 max ctx 41658, 4 requests, all claude-opus-5-5, no haiku/title calls
print per turn cum_total: 19520/39071/58651/78290 ; sum 14/98/58572/19606/78290 max ctx 19610 ; total_cost_usd 0.654877 (측정)
diff: cum total +88191 (2.13x); cache_creation 2.12x; cache_read 2.13x; output ~same. cause = base context 41.5k vs 19.5k (+~22k per request).
cost: REPL dollar 미확인 (unit price unverified). 추정 ~0.654877 x 2.1 ≈ $1.4 if cost scales with cache tokens.
unverified: what the extra ~22k REPL context holds (system prompt/tools); whether the inherited CLAUDE_CODE_CHILD_SESSION etc. in print run changed its prompt size; whether print total_cost_usd includes calls outside JSONL; >4 turns / cache TTL expiry behavior; repl exit code not captured.
---
