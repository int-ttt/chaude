# TTTT CLAUDE.md split review
verdict: PASS (1 caveat on check 1 baseline)
1 size: 33871 <= 61440 PASS. lean block: lines 1-28 intact at top, BOM, CRLF. caveat: HEAD:CLAUDE.md (LF, 325872 B) has NO lean block (chaude install added it to working tree after HEAD) -> "identical to HEAD" not checkable; block not compared against chaude template.
2 headings/lines: HEAD LF->CRLF baseline, lean block excluded. 52 ##/###/#### headings each exactly 1x across outputs; 0 non-blank HEAD lines missing; 0 non-heading lines duplicated in >1 output. New lines only: index heading+intro+8 index bullets, 8 "> 요약:" lines (18). PASS
3 BOM/CRLF: all 9 outputs start EF BB BF, 0 lone LF. PASS
4 index: all 8 Docs/CLAUDE_*.md named in CLAUDE.md; each Docs file line 1 = "> 요약: ..." then blank. PASS
5 git status: M .gitignore, M CLAUDE.md, ?? .claude/, ?? notes/, ?? 8 Docs/CLAUDE_*.md. Nothing else (ignored .claude-backup-*, .vs etc. are gitignored). PASS
6 kept sections: one-liner, 작업 규칙, Unity 작업 시 주의 (인코딩 under it, line 84), 아키텍처 원칙, 코드 구조, 색인 present. Also kept (beyond spec, small): 현재 상태 1.7K + 사망 처리 0.7K, Unity MCP 3.6K, 알려진 함정 8.9K, 개발 순서 2.1K, 하지 말 것 1.4K, 미결 0.4K. Nothing bulky left; largest section 알려진 함정 8.9K. PASS
defects: none concrete. note: Docs file names are topic-split, not date-split as spec examples (spec said "e.g.").
