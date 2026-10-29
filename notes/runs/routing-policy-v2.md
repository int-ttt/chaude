# routing-policy-v2
changed: haiku-scout, haiku-researcher effort high; descriptions (scout lookup owner, reviewer=검토, analyst=분석); agent-model-pin.py allows subagent->sonnet-reviewer (was haiku-* only = the block); CLAUDE.md routing + conductor lines; classifier.go delegateHint; DECISIONS.md entry
verified: go build/vet/test -race ok (before desc edits; Go unchanged since); grep effort ok
unverified: live hook call from real subagent (stdin test printed nothing, maybe lean_off); classifier live behavior; effort frontmatter honored by runtime; hint string only in prompt, no unit test on it
