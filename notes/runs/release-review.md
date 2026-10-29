# release-review v0.7.1
verdict: FIX (minor; not a hard BLOCK)
build/vet/test: ok (go build, go vet, go test ./... pass; cached)
secrets: none found (grep sk-ant/keys/tokens in diff, notes, LESSONS)

issues:
- med  .claude/agents/haiku-scout.md:6 -> effort: medium but description + DECISIONS + CLAUDE.md say "effort high". Fix: effort: high (researcher already high).
- med  internal/harness/classifier.go switchSession (~l.205) -> `r.pendingSnapshot = ""` drops a pending /resume import snapshot (LESSONS.md line 3 says it must not be overwritten). Fix: keep pendingSnapshot when set (only clear after a rotate-handoff), or carry it into the new session.
- med  classifier.go switchSession -> haiku->other-haiku worker switch (a.Model==model) closes current Haiku session without parking its id; that session is lost (no way back). Fix: park/pool it, or accept and document.
- low  classifier.go pickTarget -> kind param unused (pure wrapper of pickModel). Fix: drop or use.
- low  repl.go noteEdit(r.sess.SessionID...) -> SessionID may be empty before init event; noteEdit ignores "" silently, fine, but first-edit turns may not register as candidate.
- low  tui.go usageFetch -> no timeout on `claude -p /usage`; hang leaves goroutine, no feedback. Fix: exec.CommandContext 60s.
- low  tui.go busy /usage -> m.after() then async usageSend; confirm usageMsg while busy doesn't clear m.gotText (done via !m.busy guard). ok.
- low  scope: classifier now always Haiku main; --model default semantic changed (documented in DECISIONS). Version bump to 0.7.1 not yet in diff (check release file/tag).

stray/commit hygiene:
- notes/metrics/2026-10-28.jsonl (modified) and 2026-10-29.jsonl (new, 106 lines): runtime metrics, tracked previously by repo; contain session ids, no prompts. Optional: don't commit with release.
- notes/runs/* (41 untracked): working notes; repo tracks notes/runs, so ok, but skim for v0.6.10-* leftovers (fix-queue, final-review) that are obsolete.
- notes/LESSONS.md: new, 2 lines, ok to commit (ensure CLAUDE.md references it if intended).
- haikupool.go, usagebusy_test.go: ok to commit, no issues.
