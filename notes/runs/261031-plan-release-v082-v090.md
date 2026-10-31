goal: user "v0.8.2 commit and release first; then keep the current version as v0.9.0, push, release v0.8.2, merge, and release"
interpretation: commit+release v0.8.2 on master (once); push caveman-impl (v0.9.0); merge caveman-impl into master; release v0.9.0. origin/main (default branch, stale at 0.7.2) is NOT touched; releases come from master as v0.8.0/v0.8.1 did.
blocker: opus-architect could not call sonnet-reviewer (hook agent-model-pin: sonnet/opus subagents may call haiku-* only). Main thread must run step 1.

done (261031, opus-architect via haiku):
- cmd/chaude/main.go version 0.8.1 -> 0.8.2; CHANGELOG 0.8.2 last bullet -> "- cmd/chaude version 0.8.2" (build-release.sh:17 also injects -X main.version=${VERSION})
- tests on master working tree: go test ./... ok; python3 -m unittest discover -s tests/hooks -p 'test_*.py' Ran 10 OK
- tests on claude-harness-impl: go test ./... ok; tests/hooks missing there (run after merge on master)
- local commit master 82122210e9299e601b886446e655f05e428eb48a "0.8.2: sonnet/opus 서브에이전트 파일·셸 차단, haiku-prompter, usage poll/turn spend" (331 files, +16613/-112). NOT pushed, NOT tagged.
- left untracked on purpose: .DS_Store, scripts/.DS_Store, imports/ (usage zips), notes/runs/261029-review-v080-final.md (contains user email)
- merge dry-run master x caveman-impl: 6 conflicts (see step 5)

steps:
1. [main -> sonnet-reviewer, 2 calls in parallel] (a) `git -C /Users/leekwangseok/PycharmProjects/claude-harness show 8212221` focus code/hooks/agents/settings.json, skip notes/; (b) `git -C /Users/leekwangseok/PycharmProjects/claude-harness diff master~1...caveman-impl` (merge-base 529da82). Reply VERDICT PASS/BLOCK. BLOCK -> stop, report to user.
2. [haiku-runner] v0.8.2 release, cwd /Users/leekwangseok/PycharmProjects/claude-harness:
   git push origin master
   git tag -a v0.8.2 -m "v0.8.2" && git push origin v0.8.2
   wait for workflow: gh run list --workflow release.yml --limit 1; gh run watch <id> --exit-status
   footer: gh release view v0.8.2 --json body -q .body > /tmp/rel-v082.md; printf '\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n' >> /tmp/rel-v082.md; gh release edit v0.8.2 --notes-file /tmp/rel-v082.md
   return release URL (gh release view v0.8.2 --json url -q .url)
3. [haiku-runner] git -C /Users/leekwangseok/PycharmProjects/claude-harness-impl push -u origin caveman-impl (version stays 0.9.0)
4. [haiku-coder] in /Users/leekwangseok/PycharmProjects/claude-harness on master: git merge --no-ff --no-commit caveman-impl
5. [haiku-coder] resolve conflicts (all mechanical union):
   - cmd/chaude/main.go: var version = "0.9.0"
   - CHANGELOG.md: "## 0.9.0" section (caveman side, unchanged) first, blank line, then "## 0.8.2" section (master side, unchanged)
   - .claude/agents/haiku-prompter.md (add/add): master side empty -> keep caveman "Relay (U8 = c, rule_6; ...)" block as is
   - .claude/agents/opus-architect.md and .claude/agents/sonnet-analyst.md: keep master's "Plan review ..." line (the haiku-scout RETURN: verbatim / haiku-batch-worker wording), then blank line, then caveman's whole "## Caveman briefs and returns" section unchanged
   - CLAUDE.md: line 1 = caveman version ("Korean, 존댓말 for user-visible text only (haiku-tui output incl. L0); agent-to-agent briefs/returns use caveman format (docs/claude-lean/CAVEMAN_FORMAT.md). Main thread has no work tools (hook-enforced). Minimize main-thread tokens."); Flow bullet = master version verbatim (keeps "(TUI output pane, 기본값)", "결과는 파일 경로만 전달 금지; 파일로 출력하지 말고 TUI에 내용을 직접 출력", "Tests, builds, verification, reviews and file edits belong to Haiku/Sonnet agents, never to Opus."); if caveman side adds any further sentence after that, append it. Any other hunk: union, master wording wins where the same sentence differs.
   - if a hunk is not covered above and is not a pure union, STOP and report (not mechanical)
6. [haiku-checker] grep -rn '^<<<<<<<\|^=======$\|^>>>>>>>' --exclude-dir=.git . (none in tracked files); python3 -m json.tool .claude/settings.json; go test ./...; python3 -m unittest discover -s tests/hooks -p 'test_*.py'; grep -c '2500slee' on staged diff = 0
7. [haiku-coder] git commit -F msg:
   merge: caveman-impl v0.9.0 (caveman S0~S5, U8~U10)

   Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>
8. [main -> sonnet-reviewer] optional quick check of merge resolution: git show --cc HEAD (6 files)
9. [haiku-runner] v0.9.0 release: git push origin master; git tag -a v0.9.0 -m "v0.9.0" && git push origin v0.9.0; watch workflow; append same footer to v0.9.0 notes (as step 2 with /tmp/rel-v090.md); return URL.

acceptance: gh release list shows v0.9.0 Latest and v0.8.2; both bodies end with the 🤖 line; `chaude --version` from release assets reports tag version; no user email in any commit (git log -p v0.8.1..v0.9.0 | grep -c 2500slee = 0).
must NOT change: origin/main; untracked files listed above; tags v0.8.1 and older; commit messages must not contain the user email.
open_q (user): should origin/main (default branch, at 0.7.2) also be fast-forwarded to master? Not done.
