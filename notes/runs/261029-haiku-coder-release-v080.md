task: release v0.8.0 (U-rel-1 + U-rel-2), user-approved
agent: haiku-coder
date: 261029
status: done · commit + annotated tag created · NOT pushed
plan: notes/runs/261029-plan-multiproc-finish.md (§4 U-rel-1, U-rel-2)
review: notes/runs/261029-review-multiproc-v080.md (verdict PASS, blocking 0)

changed (pre-commit):
- cmd/chaude/main.go:16 version "0.7.2" -> "0.8.0"
- CHANGELOG.md: new top section "## 0.8.0" (4 bullets + size tag/5h status line), Korean, matches 0.6.x format
- no other source edits

checks (run after edits, before commit):
- go build ./... : exit 0
- go vet ./... : exit 0
- go test ./... -count=1 : exit 0 ("ok chaude/internal/harness"; other pkgs "no test files")
- go run ./cmd/chaude --version : "chaude 0.8.0"

commit:
- hash: 5e415b50b62dd928ca4d58ef6cfd0bec1c54989a
- subject: 0.8.0: per-model live child processes, idle rotate at 3599s, cache TTL observe
- trailer: Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>
- staged: 26 files, 2046 insertions, 39 deletions
  - code: cmd/chaude/main.go, CHANGELOG.md, internal/harness/{cachettl_test,children,classifier,core,dispatch,repl,tui}.go, new idlesweep.go, livechild.go, multiproc_s_test.go, switchchild.go
  - notes: notes/LEDGER.md, notes/archive/261029-plan-multiproc-finish.orig.md
  - notes/runs (multiproc/release only): haiku-coder-multiproc-finish, haiku-coder-multiproc-integration, plan-multiproc-finish(+review), plan-multiproc-rebind(+review), plan-parallel-load(+review), plan-release-v080, review-multiproc-v080, sonnet-analyst-multiproc-s-units

tag:
- v0.8.0 annotated, message "chaude 0.8.0", points to 5e415b5 (verified via rev-parse)
- plan §4 said tag only on user request; user explicitly requested the tag in this task.

unstaged (untracked, 30; not part of multiproc 0.8.0 work, left as-is):
- notes/research/billing.md
- notes/runs/261029-research-billing.md, 261029-haiku-research-billing.md
- notes/runs/261029-research-budget-flag.md (0.8.x follow-up U9-budget)
- notes/runs/261029-haiku-router-30-samples-list.md, 261029-router-replace-evalset.md
- notes/runs/261029-plan-router-{3tier,context-split,haiku-policy,replace}.md (+ 4 -review.md)
- notes/runs/261029-report-router-multiclient-v080.md
- notes/runs/261029-test-router-{30-prod-args,3tier-30,injection-minimal-vs-full,sonnet-vs-haiku}.md
- notes/runs/261029-analysis-{proposed-arch-direction,proposed-arch,task-org-two-opus-vs-one,test-org-two-opus-vs-one}.md
- notes/runs/261029-explore-{harness-readout,open-release,prompt-injection-inventory}.md
- notes/runs/261029-plan-task-isolation.md
- notes/runs/261029-test-effort-haiku-vs-sonnet.md, 261029-test-parallel-terminals.md, 261029-test-remaining.md
- judgment: router/analysis/explore/billing notes are other workstreams; if they belong to this release, stage them separately.
- this log file itself: notes/runs/261029-haiku-coder-release-v080.md (written after commit; unstaged)

doubts:
- go test ran with -count=1; test output not separately archived beyond tail of stdout.
- commit includes notes/LEDGER.md diff containing sonnet-analyst log lines (agent-appended).
- CHANGELOG 0.8.0 bullets describe flags --idle-rotate (TUI only) and --cache-ttl per plan; not re-verified against --help text.
- no push performed.
