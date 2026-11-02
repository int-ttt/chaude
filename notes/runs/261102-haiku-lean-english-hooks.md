status: done
workstream: lean-english
scope: .claude/hooks/*.py model-injected strings -> English
changed:
- .claude/hooks/style-adapter.py (+8/-2 comments, [update] instruction English, polite-form Korean reply kept)
- .claude/hooks/correction-guard.py (RULES comment only; message text was already English)
- .claude/hooks/ledger.py ("(설정)" -> "(config)", " ⚠ 이름과 다름" -> " ⚠ name mismatch")
- .claude/hooks/session-context.py (_NOTE and 3 upd strings -> English; 절약모드 trigger literal kept)
- .claude/hooks/context-alert.py ([ctx] instruction -> English)
- .claude/hooks/caveman-format.py ("필수 필드 누락" -> "missing required fields"; comments)
- .claude/hooks/haiku-context-guard.py (DENY_MSG -> English)
- .claude/hooks/agent-model-pin.py (3 deny reasons -> English; "E gate:" prefix kept)
- .claude/hooks/_lean.py (_PING comment only)
- tests/hooks/test_caveman_format.py (2 asserts updated to "missing required fields")
not changed: .claude/hooks/main-thread-guard.py (see Hangul left)
diffstat: 10 files, 35 insertions(+), 24 deletions(-) (.claude/hooks + tests/hooks)
checks:
- py_compile each changed file -> PASS (10/10)
- python3 -m unittest discover -s tests/hooks -p 'test_*.py' -> Ran 35 tests OK
- echo sample UserPromptSubmit JSON | python3 .claude/hooks/style-adapter.py -> exit 0
- echo sample UserPromptSubmit JSON | python3 .claude/hooks/correction-guard.py -> exit 0
hangul_left:
- style-adapter.py block() reasons (~L74,78-80,113,131): user-facing UserPromptSubmit reasons, not model-injected
- style-adapter.py regex lists (CORRECTION, IMPERATIVE, QUESTION, DECLARATIVE, STATUS, short approvals, 다시/재시도): match user input; English comments added
- style-adapter.py "추천:" / "다음 할 일:" in last_reply_asks(): parsed from model output
- correction-guard.py "이해: <target>에 <action>", 맞습니다/일부/아닙니다: output format in .claude/skills/correction/SKILL.md
- correction-guard.py Korean regexes: match user input
- caveman-format.py "존댓말", "caveman_leak" labels: tests/hooks/test_caveman_format.py:165,180 and metrics match on them
- session-context.py "절약모드" literal: trigger word the user types
- main-thread-guard.py deny reason "일반모드" and docstring (L4): trigger word; docstring not translated (file untouched)
- _lean.py _PING regex and docstring: match user input
doubts:
- main-thread-guard.py docstring L4 still Korean; brief asked for English comment. Left untouched to keep the diff minimal.
- style-adapter block() messages stay Korean by decision; confirm if user wants them English too.
