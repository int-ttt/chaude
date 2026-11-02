# from-claude full view + guide facts
status: done
changed:
- src/main/intt/internal/harness/importview.go (new)
- src/main/intt/internal/harness/importview_test.go (new)
- src/main/intt/internal/harness/importclaude.go (View, Subagents, dry-run view_turns)
- src/main/intt/internal/harness/repl.go (startView field)
- src/main/intt/internal/harness/tui.go (importBlocks, importedMsg, runTUI)
- src/main/intt/internal/harness/resume.go (importedMsg{imp.View})
- docs/guide/cli.md (--cwd row, --from-claude scope)
- docs/guide/repl.md (auto row, 5-min poll line)
check: go build ./... PASS; go test -count=1 ./... PASS (harness ok)
facts:
- cwd default: empty -> os.Getwd (core.go cmd.Dir="", childKey, claudeProjectDir, recordDir)
- auto: claude launched --permission-mode auto (core.go:146,153); harness modeAuto allows all prompts (mode.go decideMode); not bypass
- 5m poll: usagePollEvery=5m (usagepoll.go:17); claude -p /usage; parsed session_pct/week_pct go to metrics stage=usage only, not screen
- 5h on screen: repl.go usageLine, from rate_limit_info five_hour (not the poll)
- 5m cache TTL is separate: observedTTL from usage.cache_creation.ephemeral_5m_input_tokens (core.go:84)
doubts:
- only 2 "사용자 확인 필요" markers in docs/guide (cli.md:95, cli.md:137); third not found in repo except plan/ and .claude/ (out of scope)
- claude's own auto-mode classifier behavior not verifiable from code
- TUI rendering of view not visually checked; plain mode shows no view
- Files list still counts main-file edits only
