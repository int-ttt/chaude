request: drop Python; per-OS native impl (mac/linux bash, windows PowerShell)
found: python files to port = 9 hooks (~700 lines: style-adapter 198, session-context 161, ledger 104...), 7 scripts (~560), harness REPL+stream-json driver (~330 + test). Total ~1600 lines x 2 impls (bash, ps1) = ~3200 lines rewrite
env: jq present on mac; pwsh absent -> ps1 cannot be run, syntax-only; all hooks parse JSON stdin/jsonl transcripts (bash needs jq as new dependency)
conflict: earlier turn user said "공통 모듈 유지" (shared code once) vs now "파이썬 쓰지 말고 OS별로 따로" -> logic duplicated in 2 langs; contradicts "공통 로직 중복 금지"
risk: mac existing behavior (hooks, tests) would be rewritten -> "mac 동작 깨지 말 것" at risk; style-adapter/session-context heavy logic, REPL threading/queue in bash is impractical
options:
 A: hooks only -> bash(+jq) for mac/linux, ps1 for windows (9 files x2); keep scripts + harness REPL in python or drop them
 B: A + scripts ported too (16 files x2)
 C: everything incl. REPL (not recommended; REPL w/ streaming child process poor fit for bash)
 D: revert to current (python common + OS-specific settings/install), already done
need decision: scope A/B/C/D; keep jq dependency OK?; mac hooks switch from python to bash OK despite regression risk?
current state: python-based OS split from earlier turns remains in tree, uncommitted; nothing ported
