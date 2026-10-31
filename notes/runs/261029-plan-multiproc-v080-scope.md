# plan: v0.8.0 scope add (U8b, U5c, U9) + release
date: 261029
status: plan only. no source edited, no commit, no tag move, no push.
base: HEAD 5e415b5 "0.8.0: per-model live child processes..." ; tag v0.8.0 = annotated tag object 442e768 -> peels to 5e415b5 (local only, unpushed).
sources: 261029-plan-multiproc-finish.md (§0 l.13, §4 U-rel, §8 l.217), 261029-plan-multiproc-impl.md (§2 l.51 DesignPath, §2 l.55 reconcile, §4 l.90-105 tests), 261029-sonnet-analyst-multiproc-s-units.md, 261029-haiku-coder-release-v080.md, notes/DECISIONS.md.
gaps: see §5. U8b T6/T7 and U9 budget specs are NOT defined in the listed sources; sonnet-analyst must read 261029-research-budget-flag.md and 261029-plan-multiproc-finish-review.md before S units start.

## 0. scope decision
- version: stays 0.8.0 (tag v0.8.0 unpushed; user: move tag to the new commit). no bump to 0.8.1.
- CHANGELOG `## 0.8.0`: amend bullets for U5c, U8b, U9 only after each unit PASS (see R2).
- new commit on top of 5e415b5 (no amend of 5e415b5). tag v0.8.0 deleted and recreated on the new commit.
- untracked notes/* and notes/metrics/*.jsonl stay out of the release commit (finish §8 l.224). Stage by explicit path.

## 1. unit table
legend: H = Haiku-executable. S = sonnet-analyst (touches repl.go / tui.go / classifier.go, gate rule finish §1 G1 + sonnet-analyst split).

| ID | type | files | functions / names | depends |
|---|---|---|---|---|
| U5c | S | internal/harness/clients.go (DesignPath field, exists per impl §2 row schema), internal/harness/repl.go (runTurn Write/Edit branch ~repl.go:452-463, next to r.pool.noteEdit), idleEvent tool_use path | recordDesignPath(model, filePath string) (new, name proposed); tests TestDesignPathRecorded, TestDesignPathMissingSkipped | U5a (done), U-rel PASS |
| U8b | S | internal/harness/repl.go (start(), near r.agents.interruptRunning ~repl.go:903; loop() defer ~repl.go:810), internal/harness/tui.go (/clear handler) | reconcile wiring: r.reg.reconcile(now, deadline) at start; /clear -> children.CloseAll() (T7; spec missing, see §5) | U8 (done), reconcile method in clients.go (exists, U1a) |
| U9 | S | internal/harness/budget.go (new), cmd/chaude/main.go or repl.go (flag --max-budget-usd), core.go (passes flag to live calls) | budget ledger $3 per task, flag --max-budget-usd (names proposed; spec in 261029-research-budget-flag.md) | none (independent of U5c/U8b), G4 cost gate for live test |

H items: none of the three units is H (all touch repl.go / tui.go / need design). Haiku steps in §3 are the release/check steps only.

## 2. per-unit acceptance
common: `cd /Users/leekwangseok/PycharmProjects/claude-harness && go build ./... && go vet ./internal/harness/ && go test ./internal/harness/... -count=1` exit 0 (no "(cached)").

- U5c: `go test -count=1 ./internal/harness/ -run 'DesignPath' -v` PASS (both tests). Rules: opus active model + Write/Edit file_path matches `*/notes/runs/*-plan-*.md` and not `*-review*.md` -> DesignPath = absolute path (last write wins). Missing or 0-byte file at attach time -> skip attach, metrics `{"stage":"design_path","missing":true,"path":...}` only, no user error, no dispatch block. Attach line: `설계 파일: <abs path> (먼저 읽고 이어서 진행)` appended to pendingSnapshot end. Non-opus model: no record.
  - manual: none required.
- U8b: `go test -count=1 ./internal/harness/ -run 'Reconcile|ClearCloseAll' -v` PASS. TestReconcileKilledOwner (impl §4) must PASS; add TestReconcileStartupWired (new). Rule: State=live && !pidAlive(OwnerPID) -> parked if SessionID!="" && now-LastAPI<deadline else expired; EndedReason="harness-killed"; live owner rows untouched. Reconcile runs once at start, not per tick.
  - `go test -race -count=1 ./internal/harness -run 'ScrollBottom|WheelScroll'` (finish §6 E5) PASS (tui.go touched).
  - manual: `kill -9 <chaude pid>`; 20s later `pgrep -f 'claude -p --input-format stream-json'` = 0; restart -> clients.json row State=parked EndedReason=harness-killed.
- U9: `go test -count=1 ./internal/harness/ -run 'Budget' -v` PASS. Flag `--max-budget-usd` default 3 per task. Enforcement per impl §4: pass `--max-budget-usd` if `claude --help` lists it, else harness counter over usage + total_cost_usd, hard stop (Close child, report). Counter is per task, not summed across tasks.
  - live check: only with user OK (G4). First a $0.01-class smoke call. Not part of the acceptance above.
- release R1-R4: §3.

full gate before any commit: `go build ./... && go vet ./... && go test ./... -count=1` exit 0.

## 3. [haiku] steps
Ordered. Haiku runs H steps only. S units go to sonnet-analyst (§4).
1. H baseline: `go build ./... && go vet ./... && go test ./... -count=1` on HEAD 5e415b5 tree; record exit codes in notes/runs/261029-check-v080-scope-baseline.md (terse key: value). Stop if any FAIL.
2. [sonnet] U5c (S, §4) — wait for PASS report.
3. [sonnet] U8b (S, §4) — wait for PASS report.
4. [sonnet] U9 (S, §4) — wait for PASS report. (may run after step 1 if sonnet capacity allows; still before R1)
5. H R1 CHANGELOG: edit CHANGELOG.md top `## 0.8.0` section; add one bullet each for DesignPath (opus 설계 파일 첨부), idle/clear orphan cleanup (launch 시 orphan 정리, /clear 시 live 자식 정리), 비용 상한 $3 (--max-budget-usd). Keep Korean, match existing 0.8.0 bullets. Do not touch 0.6.7-0.7.2 gap. Forbidden words: "multi-client", "동시 실행". Version stays `0.8.0` (cmd/chaude/main.go:16 unchanged).
6. H R1 verify: `go run ./cmd/chaude --version` prints `chaude 0.8.0`; `go build ./... && go vet ./... && go test ./... -count=1` exit 0.
7. H R2 commit: `git add CHANGELOG.md cmd/chaude/main.go internal/harness/ <new budget.go>` (explicit paths only; do NOT `git add -A`; do NOT add notes/metrics or untracked notes/*). Subject: `0.8.0: design path attach, orphan cleanup on start and /clear, $3 task budget cap`. Final line: `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`. Use `git commit -F <msgfile>` (msgfile outside repo, e.g. /tmp/v080-msg.txt).
8. H R3 tag move: `git tag -d v0.8.0` then `git tag -a v0.8.0 -m "chaude 0.8.0" <NEW_SHA>` (NEW_SHA from `git rev-parse HEAD`). Verify `git rev-parse v0.8.0^{commit}` = NEW_SHA.
9. H R4 no push: confirm `git status -sb` shows ahead of origin; never run `git push` or `git push --tags`.
10. H R4 record: notes/runs/261029-haiku-coder-release-v080-scope.md (terse key: value: new sha, tag sha, test counts, changed paths).

Haiku fails a step 2x -> stop and report, do not improvise.

## 4. [sonnet] S units (sonnet-analyst; reviewer sonnet-reviewer; 2 fails -> opus-architect)
- S1 U5c: read impl §2 DesignPath + §4 tests, runTurn repl.go:440-470. Implement recordDesignPath + attach at next turn. Acceptance §2.
- S2 U8b: read finish §2 registry cleanup (l.55), impl §2 reconcile, sonnet-analyst s-units doc (startup order). Wire reconcile at start; /clear -> CloseAll (spec for /clear ambiguous, §5 item a). Acceptance §2.
- S3 U9: read 261029-research-budget-flag.md first. Implement budget.go + flag + enforcement. Acceptance §2. Live run only after G4.
- order: S1 -> S2 -> S3 (serial, shared repl.go). S3 may run in parallel with S1 only if file sets do not overlap; repl.go overlaps, so serial.

## 5. ambiguities / needs user decision (not decided here)
a. U8b T7 "/clear CloseAll": spec not in listed sources. Ambiguous: does /clear close only parked children, or all live children incl. the active one? finish §2 says CloseAll is for exit. Pick before S2; default proposal = close parked children only, keep active (matches /clear = new session without handoff, DECISIONS:42 "/clear with running agents = keep current behavior").
b. U8b T6 "reconcile orphan check": referenced as T6 in finish §0/§8 but T6 definition not in listed sources (check 261029-plan-multiproc-finish-review.md). Treated as reconcile wiring per impl §2.
c. U9 "budget" name clash: impl doc uses "U9" for the version bump (0.7.3, superseded). In this plan U9 = finish §0 "U9-budget". The version bump is R1-R3 here.
d. U9 live cost: G4 approval needed before any live call. Budget spec (`--max-budget-usd` vs harness counter) is [가정] in impl §4; verify with `claude --help` first.
e. Tag move: rewrites an unpushed local tag (user instruction). OK only if no one else fetched v0.8.0; nothing pushed so far.
f. CHANGELOG 0.8.0 already describes 3599s and TTL; R1 adds bullets, does not rewrite the existing ones. If U8b slips, drop its bullet (finish §8 l.220).

## 6. doubts
- sources state U5c/U8b/U9 only as one-line names; exact file/function names beyond §1 are proposals (marked "names proposed").
- v0.8.0 CHANGELOG bullet "--idle-rotate (TUI only)" not re-verified against --help (haiku-coder-release-v080 doubt).
- test -count=1 full run has not been archived in notes for 5e415b5; baseline step 1 fixes that.
