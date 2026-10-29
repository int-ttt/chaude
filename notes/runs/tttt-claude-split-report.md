# TTTT CLAUDE.md split report
before: 333932 bytes (BOM+CRLF, 1834 lines)
after CLAUDE.md: 33871 bytes (limit 61440)
kept in CLAUDE.md: 원문 1-148 (lean block, 규칙, 구조), 149-178 현재 상태, 1650-1834 (Unity MCP, 알려진 함정, 개발 순서, 하지 말 것, 미결); new: 색인 섹션 only

## file map (bytes)
- Docs/CLAUDE_Sandbox.md: 26940
- Docs/CLAUDE_네트워크.md: 49216
- Docs/CLAUDE_연출_VFX.md: 47335
- Docs/CLAUDE_챌린지_리플레이.md: 31861
- Docs/CLAUDE_카드_낚아채기_반격_명상_난무.md: 41727
- Docs/CLAUDE_카드_되감기_독무_사격.md: 22723
- Docs/CLAUDE_카드_키워드_도탄_역풍_충전.md: 29972
- Docs/CLAUDE_카드_포식_저격_함정_탐지.md: 52516

## heading check
script: notes/runs/tttt-verify.py, tttt-claude-split.py
orig ## to #### headings: 52 (all distinct); new total across CLAUDE.md+Docs: 53 (52 + new index heading); per-heading count mismatch: none (all exactly 1)
orig lines missing in new files (multiset compare): 0
BOM + pure CRLF: asserted on all 9 outputs

## git
 .gitignore |    9 +
 CLAUDE.md  | 1515 ++----------------------------------------------------------
 2 files changed, 52 insertions(+), 1472 deletions(-)
 M .gitignore
 M CLAUDE.md
?? .claude/
?? Docs/CLAUDE_Sandbox.md
?? "Docs/CLAUDE_\353\204\244\355\212\270
?? "Docs/CLAUDE_\354\227\260\354\266\234
?? "Docs/CLAUDE_\354\261\214\353\246\260
?? "Docs/CLAUDE_\354\271\264\353\223\234
?? "Docs/CLAUDE_\354\271\264\353\223\234
?? "Docs/CLAUDE_\354\271\264\353\223\234
?? "Docs/CLAUDE_\354\271\264\353\223\234
?? notes/
note: .gitignore, .claude/, notes/ were modified/untracked by chaude install, not by this task. Docs/작업정리_*.txt untouched. Not committed.
