task: haiku-context-guard.py Bash deny check
compile: py_compile exit=0
git diff -- .claude: haiku-context-guard.py (docstring, import re, BASH_READ/BASH_WRITE regex, sonnet-/opus- Bash deny block); settings.json matcher Read -> Read|Bash
cases: 15, match: 15, mismatch: 0
| # | agent_type | command | expected | actual |
| 1 | sonnet-analyst | cat a | DENY | DENY |
| 2 | sonnet-analyst | grep -r x . | DENY | DENY |
| 3 | sonnet-analyst | echo hi > o | DENY | DENY |
| 4 | sonnet-analyst | echo hi >> o | DENY | DENY |
| 5 | sonnet-analyst | ls \| tee x | DENY | DENY |
| 6 | sonnet-analyst | sed -i s/a/b/ f | DENY | DENY |
| 7 | sonnet-analyst | FOO=1 head -5 f | DENY | DENY |
| 8 | sonnet-analyst | ls && tail f | DENY | DENY |
| 9 | sonnet-analyst | git status | PASS | PASS |
| 10 | sonnet-analyst | ls 2>&1 | PASS | PASS |
| 11 | sonnet-analyst | x 2>/dev/null | PASS | PASS |
| 12 | sonnet-analyst | git diff --stat | PASS | PASS |
| 13 | opus-architect | awk 1 f | DENY | DENY |
| 14 | haiku-scout | cat a | PASS | PASS |
| 15 | haiku-scout | echo hi > o | PASS | PASS |
script: /tmp/hooktest/run_guard.py (outside repo)
