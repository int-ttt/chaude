# v0.8.1 release gate: BLOCKED by reviewer
status: implemented A/B/C, NOT committed/tagged/released
done:
- sonnet no-direct-read: sonnet-analyst.md tools -> Write,Bash,WebSearch,WebFetch,Agent; hook deny Read/Grep/Glob for sonnet-analyst
- output mode: CLAUDE.md L5/L15/L21 edited; repl.go empty-reply fallback sends result to TUI
- resume: drain.go onUserEvent + repl.go call already in worktree (verified present, tests ok)
- opus-architect Agent-only: present
- version 0.8.1 (cmd/chaude/main.go), CHANGELOG 0.8.1 section
checks: go build/vet/test ./... PASS; chaude --version 0.8.1
review: notes/runs/261029-review-v081.md verdict FAIL (1 blocking)
blocking: classifier.go:15 routerModelDefault sonnet->haiku (+3 test files: classifier_model_test, conductor_test, routerprompt_test) in dirty tree; not in v0.8.1 scope, not in CHANGELOG, old comment "pending user approval"
open decision (user): (a) revert router flip and exclude from v0.8.1 (needs hunk-level revert of classifier.go + 3 tests), or (b) approve it and add to CHANGELOG
non-blocking: CHANGELOG omits CLAUDE.md policy changes; "Edit removed" wording overstated (Write+Bash remain); hook Grep/Glob deny dead (matcher Read only); panelWheel clamp loose
not done: commit (trailer Co-Authored-By: Claude Haiku 5.5), tag v0.8.1, release
