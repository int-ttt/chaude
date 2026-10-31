# Plan: model-call policy rewrite (router removed, Haiku main forwards to Opus)

status: draft v1 (design only, no code changed by this plan)
date: 2026-10-31
repo: /Users/leekwangseok/PycharmProjects/claude-harness (original repo, not a copy)
base: HEAD 248b5e0 (master). Uncommitted worktree changes are archived first, not reused (see S0)
source_request: notes/pastes/0e23e5b9.md (last user turn, routing opus tier)
structure_source: plan/caveman-structure.md (v7)
review: opus-architect review (workflow step 2) NOT run by this plan author; run it before S1

## 0. Scope

- in: remove classifier/router completely; main session = Haiku; every non-local user prompt goes Haiku main -> opus-architect (forward verbatim + one-line summary); rotation thresholds haiku 80000, opus 200000; REPL shape and session management kept; rebuild message formats from plan/caveman-structure.md.
- out: REPL/TUI layout, session handoff/drain/idle logic, metrics schema beyond removing router fields, price/cost measurement, release/tag/push.
- rule: never delete originals. "Discard" = move to archive/261031-model-policy-old/ (same relative path), verified before any removal from the tree.

## 1. Facts found (path:line, read-only check 261031)

- F1: repl.go:27-29 already defines haikuThreshold = 80_000, opusThreshold = 200_000.
- F2: repl.go:865-867 thresholdFor(model): opus -> opusThreshold, else r.a.Threshold (flag default haikuThreshold, repl.go:1069).
- F3: repl.go:62 Conductor bool (default true, flag repl.go:1084); repl.go:1205-1208 setConductor; repl.go:1209 conductorRule text (router rules).
- F4: classifier.go (30 lines) does NOT contain the classifier. It holds switchSession(model, resume), used by model switch / rotation. It must be kept (moved), not archived with the router.
- F5: haikupool.go (53 lines) = workerPool of Haiku sessions with edit history, reused for edit-type prompts. Part of the old routing; delete candidate (see open O5).
- F6: orch.go / orchws.go + tests: A/B orchestration modes m0|b|c|d|e|f. orchDefaultModel sets per-mode main model (orch.go:23-32). repl.go:194, 249, 1093, 1100-1115 wire them. Model-call policy -> remove.
- F7: budget.go:42 chargeOneShot comment mentions classifier (cost booking for one-shot calls; keep function).
- F8: .claude/settings.json modified in worktree; main model currently opus per CLAUDE.md drift (CLAUDE.md:33 says main always Haiku).
- F9: worktree state (git status, 261031 start): D .claude/agents/sonnet-analyst.md, D .claude/agents/sonnet-reviewer.md, M .claude/hooks/agent-model-pin.py, M .claude/settings.json, M internal/harness/{classifier.go,conductor_test.go,haikupool.go,multiproc_s_test.go,repl.go}, D internal/harness/{classifier_model_test.go,routerprompt_test.go}, M notes/DECISIONS.md, M notes/LEDGER.md, M notes/metrics/2026-10-31.jsonl, M tests/hooks/test_orch_gate.py, ?? imports/, ?? .DS_Store, ?? scripts/.DS_Store, ?? notes/runs/261031-*.md (3 files).
- F10: stash@{0} exists (WIP 542b1e2). Not touched by this plan.
- F11: CLAUDE.md:4 "Korean, 존댓말." global; caveman O1 approved (Q1) to narrow to user-visible text; not yet applied (S5).
- F12: CLAUDE.md:9 routing names sonnet-analyst/sonnet-reviewer; CLAUDE.md:28 rotation says "Haiku main 80k, Opus and Sonnet main 270k" -> must become 80k / 200k.
- F13: CLAUDE.md:22-23 requires a trailing 추천 block. Request says no 추천 block for this work (see O3).
- F14: C13 (caveman plan): opus subagents may call haiku-* only (hook message v0.8.2). Main -> opus-architect call: verify in agent-model-pin.py (S5).

## 2. Target model-call policy (what the code must do)

- P1: main session model = haiku (fixed). `--model` default haiku. Forced `opus로`/`sonnet으로` handling: removed (router gone); see O4.
- P2: each user prompt that is not a local slash command (localcmd.go) and not an ack is forwarded by the main Haiku to opus-architect in ONE foreground Agent call. Forward text = user words verbatim + one summary line + "lang: ko" (caveman B-format fields task/paths/limits/return/lang).
- P3: opus-architect returns R-decide (caveman 3.4) with summary_ko. Main Haiku relays summary_ko to the user (haiku-tui output, Korean 존댓말). Main writes no work itself (CLAUDE.md:4-12).
- P4: opus-architect calls only haiku-* agents for work (C13). Main calls opus-architect and haiku-* and sonnet-reviewer (before finishing edits).
- P5: rotation: thresholdFor(haiku) = 80000; thresholdFor(opus) = 200000; no sonnet threshold. Rotation check after each answer (existing shouldRotate, repl.go:870).
- P6: no router, no classifier call, no conductor flag, no orch modes, no CHAUDE_ROUTER_MODEL env.
- P7: haikupool (edit-history reuse) removed with the router (O5 default).

## 3. File-level change list

### DELETE (archive first, then remove from tree)

- internal/harness/orch.go, internal/harness/orch_test.go (A/B modes; F6)
- internal/harness/orchws.go, internal/harness/orchws_test.go (ws parsing for mode f; F6) -- check callers via S1 scout first
- internal/harness/haikupool.go (F5, O5)
- internal/harness/conductor_test.go (tests conductor; already M in worktree)
- internal/harness/multiproc_s_test.go: NOT deleted; modify (see MODIFY). Listed here only as "check before edit".
- internal/harness/classifier.go: archive the file AFTER moving switchSession (F4). Do not delete switchSession.

Already deleted in worktree (archive from HEAD, do not lose):
- internal/harness/classifier_model_test.go
- internal/harness/routerprompt_test.go
- .claude/agents/sonnet-analyst.md (see O6; keep out of policy, archive)

### MODIFY

- internal/harness/repl.go
  - remove Conductor field (62), flag (1084), setConductor (1205-1208), conductorRule (1209-), orchMain (194), orchRule use (249), parseOrch/orchDefaultModel/applyOrchEnv wiring (1093-1115).
  - thresholdFor: opus -> opusThreshold; otherwise haikuThreshold. Keep constants 27-29.
  - -threshold flag: removed (O7 default). Keep -compact-window (1070) unchanged.
  - -model default: haiku.
  - metrics init (1142): drop "conductor"/classify fields only.
- internal/harness/switchchild.go or new internal/harness/switch.go: receives switchSession (F4). Check name clash with existing switchchild.go first (S1).
- internal/harness/budget.go: comment at :42 only (classifier -> generic one-shot).
- internal/harness/metrics.go / metricscmd.go: drop stage=classify and conductor fields if present (grep first).
- cmd/chaude/*.go: remove flags that no longer exist (--conductor, --orch, --threshold) if wired there.
- internal/harness/multiproc_s_test.go, unit_test.go, acceptance_test.go, cleanenv_test.go, delegation_test.go: remove references to deleted symbols (grep-driven, S1 list).
- .claude/settings.json: main model haiku; keep hook list; add caveman-format.py (S4 log mode) to PostToolUse matcher "Agent|Task".
- .claude/hooks/agent-model-pin.py: allow main -> opus-architect and main -> sonnet-reviewer; keep opus -> haiku-* only; no block on main path except existing rules.
- tests/hooks/test_orch_gate.py: archive and remove if it tests orch modes only; otherwise modify to new gate rules.
- .claude/agents/haiku-scout.md, haiku-researcher.md, haiku-coder.md, haiku-batch-worker.md, haiku-runner.md, haiku-checker.md, haiku-drafter.md: append R-read / R-write template blocks (caveman S1, S2). Single source: docs/claude-lean/CAVEMAN_FORMAT.md.
- .claude/agents/haiku-prompter.md: keep repetition-loop role (U10=b); add forward role text only if the main forward is delegated (O2 default = no, main forwards directly). So: no change except removing router wording if any.
- .claude/agents/opus-architect.md: accept forwarded user prompt; reply R-decide (3.4); B-format brief rules (caveman S3).
- .claude/agents/sonnet-reviewer.md: restore from HEAD (`git show HEAD:...`) because workflow step (4) needs it. Check file is not in tests expecting removal.
- CLAUDE.md
  - :4 scope narrowed to user-visible text (caveman O1, S5; do last).
  - :9 routing: remove sonnet-analyst path; state haiku main -> opus-architect forward.
  - :28 thresholds: "Haiku main 80k, Opus main 200k".
  - :33 chaude conductor paragraph replaced by one line: "main = Haiku; user prompt -> opus-architect (R-decide) -> Haiku relays summary_ko".
  - :22-23 추천 rule: NOT changed in this plan (O3).
- notes/FACTS.md: add 2 lines (see section 4).
- notes/DECISIONS.md: append [policy] lines (newest at bottom).

### CREATE

- internal/harness/policy.go: const mainModel="haiku", opusModel="opus" (forward target name "opus-architect"), haikuThreshold/opusThreshold moved here from repl.go:27-29 (single source), func thresholdFor(model string) int.
- internal/harness/policy_test.go: thresholdFor("haiku")==80000, thresholdFor("opus")==200000, default main model "haiku", no classifier symbol (grep test or build check).
- internal/harness/forward.go: builds forward text = verbatim prompt + summary line + B-format keys (task, paths, limits, return, lang: ko). Pure function, unit-tested.
- internal/harness/forward_test.go: verbatim preserved (Korean, multi-line, "\" continuation), lang: ko present, no truncation of prompt.
- docs/claude-lean/CAVEMAN_FORMAT.md: copy of caveman-structure.md 3.1-3.6 verbatim (S0).
- .claude/agents/haiku-tui.md: new user-facing agent (caveman U9=a; Korean 존댓말 output).
- .claude/hooks/caveman-format.py: log-mode format guard (caveman 6절), no blocking.
- tests/hooks/test_caveman_format.py: required keys, status values, truncated yes|no, claim pattern, summary_ko when lang: ko.
- archive/261031-model-policy-old/ : all archived originals + worktree-before.patch (S0).
- notes/runs/261031-model-policy-refs.md: scout output (S1), not a design doc.

### NOT TOUCHED

- internal/harness/tui.go, input.go, handoff.go, idlesweep.go, drain.go, switchchild.go logic (except F4 move), usagepoll.go, ratestatus.go, category.go (O8), notes/LEDGER.md (hook-written), notes/metrics/*.jsonl (hook-written), imports/, scripts/.DS_Store, .DS_Store, stash@{0}.

## 4. Fact propagation (old -> new)

- notes/FACTS.md add:
  - [rotate] haiku main rotation 80000 ctx tokens; opus 200000 (user, 261031)
  - [policy] router/classifier removed; Haiku main forwards each non-local prompt to opus-architect (user, 261031)
- grep -rn old values, exclude .git and logs:
  - "270" / "270k" / "270000" (CLAUDE.md:28, DECISIONS rotate lines) -> replace only where it is the rotation threshold; list others unchanged.
  - "150000" / "150,000" (old repl note) -> ambiguous, list unchanged unless rotation context.
  - "CHAUDE_ROUTER_MODEL", "classifier", "conductor", "--orch", "--threshold" -> remove in code paths; in notes/runs (historical) leave unchanged.
  - "Sonnet classifier" (CLAUDE.md:33, DECISIONS) -> historical DECISIONS lines stay; CLAUDE.md:33 replaced.

## 5. Step order for Haiku executors

Each step: one executor, one foreground call, brief <=5 lines, acceptance self-check, R-write/R-read return. Commit once per step on branch `model-policy-rewrite` (created from HEAD in the original repo). No push, no stash, no reset, no checkout over dirty files.

- S0 snapshot and archive — executor: haiku-runner
  - run: `git -C <repo> status --short > archive/261031-model-policy-old/status-before.txt`; `git -C <repo> diff > archive/261031-model-policy-old/worktree-before.patch`; copy each untracked file in the list above into the archive (cp -p).
  - archive HEAD copies of already-deleted worktree files: `git show HEAD:<path>` -> archive path.
  - accept: archive files exist; `git stash list` still shows stash@{0}.
- S1 reference scout — executor: haiku-scout (R-read, effort high, RETURN verbatim <=4000 chars)
  - grep -rn over internal/ cmd/ .claude/ tests/ CLAUDE.md for: classif|router|ROUTER|conductor|orch|threshold|switchSession|haikupool|workerPool|chargeOneShot|stage=classify
  - exclude .git, notes/metrics, notes/LEDGER.md, notes/runs historical files.
  - output: notes/runs/261031-model-policy-refs.md (path:line per hit, not_checked list).
  - accept: every hit listed with path:line; not_found and not_checked separate.
- S2 move switchSession, create policy — executor: haiku-coder (single file each)
  - move switchSession from classifier.go to switch.go (or switchchild.go if clean).
  - create policy.go (section 3 CREATE), move threshold constants there.
  - accept: `go build ./...` passes after S2 (classifier.go still present until S3).
- S3 delete router and orch — executor: haiku-batch-worker (archive first, then remove)
  - archive then remove: orch.go, orch_test.go, orchws.go, orchws_test.go, haikupool.go, conductor_test.go, classifier.go (after S2 move), classifier_model_test.go and routerprompt_test.go restored from HEAD into archive.
  - accept: archive file count matches; `go build ./...` may still fail until S4.
- S4 modify repl.go and callers — executor: haiku-coder (one file per call; repl.go is one call)
  - apply section 3 MODIFY for repl.go, budget.go, metrics, cmd/chaude; fix test references from S1 list.
  - accept: `go build ./...` and `go vet ./...` PASS; `go test ./...` PASS (see section 6).
- S5 hook and settings — executor: haiku-coder
  - agent-model-pin.py: allow main -> opus-architect and sonnet-reviewer; keep opus -> haiku-* only.
  - settings.json: main model haiku; add caveman-format.py (log).
  - tests/hooks: archive test_orch_gate.py if orch-only; add test_caveman_format.py.
  - accept: `python3 -m unittest discover -s tests/hooks -p 'test_*.py'` PASS.
- S6 agent files and format (caveman S0-S3) — executor: haiku-batch-worker for identical template blocks; haiku-coder for opus-architect.md and haiku-tui.md
  - create docs/claude-lean/CAVEMAN_FORMAT.md (S0).
  - append return template blocks to haiku-scout, haiku-researcher, haiku-coder, haiku-batch-worker, haiku-runner, haiku-checker, haiku-drafter (S1, S2).
  - opus-architect.md B-format and R-decide (S3); create haiku-tui.md (U9).
  - restore sonnet-reviewer.md from HEAD.
  - accept: each template string equals CAVEMAN_FORMAT.md block (caveman 10절 grep).
- S7 guard hook (caveman S4) — executor: haiku-coder
  - caveman-format.py log mode; registered in settings with matcher Agent|Task; writes notes/metrics/<date>.jsonl only; never blocks.
  - accept: unittest test_caveman_format PASS; hook exits 0 on malformed input.
- S8 CLAUDE.md and notes — executor: haiku-drafter? NO: haiku-batch-worker (verbatim edits), haiku-coder for CLAUDE.md lines 9, 28, 33
  - section 3 CLAUDE.md edits, S5 of caveman (O1). Keep :22-23 추천 rule unchanged (O3).
  - notes/FACTS.md and DECISIONS.md appends (section 4).
  - accept: grep "80k\|200k\|270" in CLAUDE.md shows only intended lines; no "classifier" in CLAUDE.md.
- S9 verification — executor: haiku-runner
  - run acceptance (section 6). Record outputs in notes/runs/261031-model-policy-rewrite-result.md.
- S10 review — executor: sonnet-reviewer (before finishing, per workflow step 4). It reviews the diff; Haiku fixes findings in one more round.

## 6. Acceptance commands

Run from repo root `/Users/leekwangseok/PycharmProjects/claude-harness`. (Go module root: confirm with `ls go.mod` in S1; if go.mod is under another dir, use that dir.)

- go build ./...
- go vet ./...
- go test ./...
- python3 -m unittest discover -s tests/hooks -p 'test_*.py'  (literal `python -m unittest tests/hooks` from the brief is not a valid module path; this is the working form. Confirm `python3` >= 3.8.)

Pass criteria:
- all four exit 0.
- policy_test.go asserts thresholdFor haiku 80000 and opus 200000.
- grep -rn "classif\|CHAUDE_ROUTER_MODEL\|conductor\|--orch" internal/ cmd/ .claude/ tests/ returns no live code hits (archive and historical notes excluded).

## 7. Rollback

- R-A before any change: archive/261031-model-policy-old/worktree-before.patch restores the old uncommitted worktree (`git apply` onto a clean HEAD checkout).
- R-B per step: each step is one commit on `model-policy-rewrite`; revert with `git revert <sha>`. Do not use reset --hard.
- R-C archived files: copy back from archive/261031-model-policy-old/<relative path>.
- R-D branch abandon: `git switch master` (dirty files on master are untouched, because work is on the branch). stash@{0} is never used.
- R-E hooks: remove caveman-format.py entry from settings.json to disable the guard (log only anyway).
- Never run: git stash, git reset --hard, git clean, git push, rm on originals.

## 8. Open questions and risks

- O1 (decide): base of work. Plan = branch `model-policy-rewrite` from HEAD in the original repo. caveman C8/U6 says master working tree must not be edited directly and worktree from HEAD. Default: branch in original repo, dirty files archived not reused.
- O2 (decide): forward path. User wording: Haiku main forwards to Opus. caveman Q2/U8 says haiku-prompter forwards. Default here: main forwards directly (one hop, caveman rule_3). haiku-prompter keeps only its loop role.
- O3 (decide): CLAUDE.md:22-23 requires a 추천 block on replies that propose or ask. The request says no 추천 block for this work. Default: CLAUDE.md rule unchanged in this plan; this work's reply has no 추천.
- O4 (decide): forced `opus로`/`sonnet으로` user overrides. Router gone -> default: removed (not in P1). Confirm.
- O5 (decide): haikupool.go (edit-history reuse). Default: delete with the router. If the user wants Haiku session reuse for edits, keep and move out of router code.
- O6 (decide): sonnet-analyst. Not in the requested workflow (only sonnet-reviewer is named). Default: archive it, keep sonnet-reviewer. CLAUDE.md:9 sonnet-analyst lines removed in S8.
- O7 (decide): -threshold flag. Default: removed, thresholds fixed per model (80k/200k). If kept, default must still be 80000 and opus stays 200000 regardless.
- O8 (decide): category.go ([cat:] rotation on category switch). It is session management, not model choice. Default: kept unchanged. Ambiguous; confirm.
- O9: opus-architect receiving every prompt raises Opus usage. Not measured (caveman C9: no cost data). Do not claim savings.
- O10: main -> opus-architect Agent call is allowed by hook? Unverified until S5 reads agent-model-pin.py. If blocked, stop and report, do not bypass.
- O11: 4000-char return cap (C12) vs R-decide length. Opus summaries must include truncated/omitted fields.
- O12: caveman S5 (CLAUDE.md:4 narrowing) is a global-rule change. Q1 already approved O1 (261029-261031 notes). Apply only in S8.
- O13: sonnet-reviewer restore (S6) must not conflict with the agent-model-pin rule that sonnet agents cannot touch files (CLAUDE.md:10). It reviews only, reads via haiku-scout.

## 9. Done / remaining (for compaction)

- done: read request, plan/caveman-structure.md, CLAUDE.md, DECISIONS.md, FACTS.md, harness file list and grep refs; wrote this plan.
- remaining: opus-architect review (workflow step 2, foreground, output notes/runs/261031-plan-model-policy-rewrite-review.md); merge improvements; execute S0-S10 via Haiku executors; sonnet-reviewer before finish.
- not done here: no code, no archive, no commit, no branch created.
