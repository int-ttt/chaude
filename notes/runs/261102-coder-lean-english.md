# lean-english: model-facing text to English
status: done
changed:
- CLAUDE.md: Language bullet 1 replaced (spec text); old returns sentence split to bullet 2; Roles line 6 `haiku-읽기용/쓰기용` -> `haiku-read/write role`; Reply: 23 (haiku-tui renders Korean), 24 (plain English), 26 (추천 list in English)
- src/main/intt/internal/harness/resident.go:361 viaTUI prompt -> English (Hangul left: 존댓말)
- src/main/intt/internal/harness/switchchild.go:281 `설계 파일:` -> `Design file:`
- src/main/intt/internal/harness/repl.go:41 forwardRule tail -> English
no change: category.go categoryRule (already English, ack word list kept); resident.go needWords (input match); rules/resident-prompter.md (only `아니, 근데, 말고` input list)
hangul left (intentional):
- resident.go:334 needWords, category.go ackWords/categoryRule list, resident-prompter.md:7 correction words: match user input
- resident.go:361 `존댓말`: register term for output
- CLAUDE.md: `추천:`, `haiku로`/`opus로`, `ㅇㅇ`, `아니…` trigger literals
- notices/status (resident.go:141,288-377; switchchild.go:58,63; repl.go UI): user-visible, not model-facing
check: cd src/main/intt && go build ./... && go test ./... -> build OK; harness ok, all packages ok (no FAIL)
language-setting: none found. grep language|outputStyle|locale|korean in src/main/intt (non-test .go), .claude/settings.json, .claude/ -> no matches. gen-settings = internal/settings/settings.go, harness/chaude_settings.go: no language key.
note: git status shows other modified .claude/agents, .claude/skills files not touched by me.
