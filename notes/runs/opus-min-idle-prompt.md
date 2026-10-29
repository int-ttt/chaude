# opus-min-idle-prompt
request: (1) minimize Opus main output (2) tests/most work -> haiku/sonnet (3) idle->prompt flow; keep scroll-to-bottom
change: CLAUDE.md (claude-lean block) + 2 bullets after the policy header: 대기→프롬프트 flow; scroll rule pointer
interpretation: '대기 - 프롬프트' = main thread idle/silent until user prompt -> one parallel foreground delegation round -> one final short reply -> idle. Not a Go/TUI code change (TUI already has idle reader + prompt box).
already_present: main-thread-guard hook (no work tools), routing to haiku/sonnet in CLAUDE.md; only made explicit that tests/builds/reviews/edits never go to Opus
go_code: unchanged (uncommitted internal/harness work preserved)
scroll: unchanged; TestScrollBottomOnInputAndUpdate retained
uncertainty: intent may have meant a TUI state label "대기" in the status line; not done, no evidence
