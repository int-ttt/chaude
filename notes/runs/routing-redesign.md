# routing-redesign
chosen: delegate-from-Opus (not Sonnet-main handoff)
why: handoff rotates session (cache loss + handoff tokens, flips back later); Opus main already routes/delegates per CLAUDE.md; hint costs ~0
change: classifier.go pickModel(cur,topic,size)->(model,delegate). cur opus/"" -> always opus; topic new+small -> task prefixed with delegateHint. cur sonnet (user-forced) stays unless new+large->opus. classifyPrompt: topic new = unrelated to current work incl. simple test/lookup side tasks. conductorModel const "sonnet" (asserted in test).
tests: conductor_test.go TestParseRoute, TestMainRouteSwitchPolicy (8 cases). Live test (CHAUDE_LIVE) not run; want strings now "opus".
CLAUDE.md: one line appended.
results: go build, go vet, go test -race ./internal/harness pass.
uncertain: LLM "unrelated" judgment unverified live; Opus may ignore hint; repl.go help text still says haiku|sonnet|opus (sed failed, untouched); no commit.
