# check: caveman flow A1-A5 (read-only)
status: partial
A1: FAIL (docs/claude-lean/CAVEMAN_FORMAT.md:33-41 3.3 still old: changed: <경로 목록>, diffstat, check, ws, summary_ko). plan/caveman-structure.md:92-96 new 3.3 PASS; 2.1 heading line 51 PASS; S6 line 185 PASS
A2: PASS (CLAUDE.md:10 relay; :11 쓰기 흐름:; :12 읽기 흐름:)
A3: contradictions remain (see below)
A4: PASS with gap: CLAUDE.md:11 omits checker check relay (plan 2.1 line 54 has it)
A5: PASS (git diff --stat main-thread-guard.py empty)
diffstat: plan/caveman-structure.md 34 (+/-), CLAUDE.md 12
table: plan §4 rows 35 HEAD / 35 now; 6 row pairs replaced (lines 31-35+42 -> 36-40+43), no extra/dup row. E2 "7행" wrong -> 6
stale hits (diffstat|summary_ko만|relays summary_ko):
- plan/caveman-structure.md:46 rule_2 "메인은 summary_ko만 사용자에게 전달" -> contradicts new flow
- plan/caveman-structure.md:98 change note -> ok
- docs/claude-lean/CAVEMAN_FORMAT.md:37 diffstat -> contradicts (pending S6)
- .claude/agents/{haiku-coder,haiku-batch-worker,haiku-runner,haiku-checker,haiku-drafter}.md:25-29 diffstat template -> contradicts (pending S6)
- .claude/hooks/caveman-format.py:15 REQUIRED R-write has diffstat,check,ws -> contradicts (pending S6)
- tests/hooks/test_caveman_format.py:26,73 -> pending S6
CLAUDE.md: no hits
flow.md:58-61 ref (plan:100): file not present -> dangling
docs CAVEMAN_FORMAT.md:16 unrelated 1-line diff (E<n> ref removed)
