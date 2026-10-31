# decision: branch interactive-child-compare direction (261031)
status: recommendation, awaiting user choice
scope: notes/runs/261031-scope-io-architecture-branch.md
main_plan: notes/runs/261031-plan-io-architecture-review.md
recommend: A (branch = 비교 실험·측정 한정), then B items only if measurements support
## options
A: measure only (plan 도입 전 측정 1-6, esp. 5 상주 vs 신규 child) using existing orch A/B modes (orch.go --orch m0|b|c|d|e|f, orchws.go mode f). pro: plan says measure first; infra already exists (untracked); no new child code. con: no user-visible feature this branch.
B: keep current child + instruction edits (agents/*.md Return rules, CLAUDE.md). pro: cheap. con: skips plan's 측정 먼저; adopts unverified savings.
C: implement interactive TUI child. con: plan verdict 비권장 (261030-plan-interactive-child-review.md:32: 비용·권한·알림 신호 상실); current child already keeps context (core.go:145 `-p --input-format stream-json --output-format stream-json`); no interactive child code exists yet (grep: no pty, only repl.go:140 `interactive bool` flag) -> full new build.
## why A
- plan section 도입 전 측정 is a precondition for 권장 구조; A satisfies it, B skips it, C contradicts it.
- orch.go/orchws.go/turnspend.go/usagepoll.go (untracked) already provide A/B mode + per-turn spend + usage polling = measurement harness.
- item 5 (상주 vs 신규 child) directly answers the branch name's "compare" without building a TUI child.
## uncommitted-change conflicts
- tracked diff: repl.go +132/-?, drain.go +90, ratestatus.go +23, livechild.go +2 (tracker.notify = r.agentNotice in unbindActive/bindChild), classifier/metrics/usagebar small. = 0.8.2 work (notify/drain/rate status), not interactive child.
- A/B: no overlap beyond reading these files; C: would rewrite core.go:145 args, repl.go, livechild.go -> direct overlap with uncommitted repl.go/livechild.go.
- advice: commit or stash-split the 0.8.2 changes as their own commit before branch work (main session decides; not done here).
## user decision needed
브랜치 목표를 "인터랙티브 child 구현"에서 "측정 비교(A)"로 바꾸는 것에 동의하는지. (yes -> next: plan file for measurements 1-6 with cost cap; caveman stopped at $1 cap, so cap value also needed)
