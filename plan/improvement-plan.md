# Improvement plan (2026-11-12)

Source: two read-only reviews on 2026-11-12 (grep/git/wc only; no builds or tests run).
Numbers are from those scans.
Non-test counts are heuristic (lines before first `#[cfg(test)]`).

## 1. Design document inventory

Scope: `plan/`, `docs/` (incl. `docs/claude-lean/`, `docs/guide/`), `notes/*.md`, `notes/research/*.md`, `record/*.md` (top level).
Skipped: notes/runs, notes/tmp, notes/distill, record/pastes, record/metrics.
Git date = `git log -1 --format=%ad --date=short -- <path>`.

| path | lines | topic | status | evidence |
|---|---|---|---|---|
| plan/caveman-structure.md | 58 | caveman channel format: current design (as implemented) | current | L3 `status: current design, as implemented 2026-11-12`; history in plan/archive/caveman-structure-history.md (339 lines, old design v7 and decisions) |
| plan/gui-basic.md | 40 | GUI basic design (Tauri) | superseded | L1 `> Superseded 2026-11-12 by plan/gui-parity.md (egui instead of Tauri).`; git 2026-11-11; uncommitted edit |
| plan/gui-parity.md | 46 | GUI parity plan (egui, 0.13.0) | current | L1 `# GUI parity plan (2026-11-12)`; L5 `## Decisions`; untracked (no git date) |
| plan/interactive-child-resident-sessions.md | 98 | resident sessions and interactive child feasibility | planned-only | L3 `status: 계획만 작성됨. 구현·측정 미실행. 버전: v2`; git 2026-10-31 |
| plan/rust-engine-tui.md | 24 | Rust engine and TUI design | stale | L19 "No alternate screen" vs crates/cli/src/tui/mod.rs:1 (full-screen alternate screen); git 2026-11-03 |
| plan/rust-parity-checklist.md | 113 | Go-to-Rust user-facing feature parity | done | L4 status legend; table rows: 63 `done`, 0 `differs`, 0 `missing`; git 2026-11-04 |
| plan/rust-port.md | 57 | Rust port plan and phase status | current | L1 `# chaude Rust port plan (decided 2026-11-03)`; phase status lines with open items (L52); git 2026-11-09 |
| plan/rust-test-parity-p1.md | 143 | Go-to-Rust test parity, part 1 | superseded | L3 `> Merged into plan/rust-test-parity.md`; git 2026-11-04 |
| plan/rust-test-parity-p2.md | 161 | Go-to-Rust test parity, part 2 | superseded | L3 `> Merged into plan/rust-test-parity.md`; git 2026-11-04 |
| plan/rust-test-parity-p3.md | 248 | Go-to-Rust test parity, part 3 | superseded | L3 `> Merged into plan/rust-test-parity.md`; git 2026-11-04 |
| plan/rust-test-parity.md | 384 | Go-to-Rust test parity, 320 tests | done | L11-20 totals: covered 237, partial 10, ignored 48, missing 0, n-a 25; git 2026-11-05 |
| plan/tui-parity.md | 288 | Go-to-Rust TUI parity checklist | current | L5 `Status: all items unchecked (`[ ]`)` vs 181 `[x]` and 11 `[ ]` in body; git 2026-11-05 |
| docs/long-session-bench-plan.md | 58 | long session cost efficiency test plan | planned-only | L3 `- 상태: 계획만, 미실행 (작성일 2026-11-01)`; git 2026-11-02 |
| docs/pingpong-bench.md | 96 | ping-pong game efficiency benchmark results | done | L1-5 `date: 2026-11-01`, runs 2 per method, results recorded; git 2026-11-02 |
| docs/claude-lean/APPLY_GUIDE.md | 168 | apply guide (Opus main, agents do work) | current | L23 `./install.sh <프로젝트 폴더>`; root install.sh exists; git 2026-11-02 |
| docs/claude-lean/CAVEMAN_FORMAT.md | 73 | caveman brief and return formats | current | L1 `# Caveman format (canonical)`; git 2026-11-11 |
| docs/claude-lean/CHANGELOG.md | 111 | Claude Lean change log | stale | L5 `## 0.5.3` is the newest entry vs release 0.12.5 (git a37b862); git 2026-11-01 |
| docs/claude-lean/FRESH_START.md | 91 | fresh start: extract and reset session | current | L1 `# 완전 초기화 가이드 — 중요한 내용만 남기고 새로 시작`; git 2026-11-02 |
| docs/claude-lean/MIGRATE_NOW_PROMPT.md | 31 | one-turn migration prompt for large session | current | L1 `# 기존 대형 세션 → 새 세션 마이그레이션 프롬프트`; names haiku-scout (exists in .claude/agents); git 2026-11-01 |
| docs/claude-lean/README.md | 86 | Claude Lean overview and install | current | L1 `# Claude Lean`; git 2026-11-12 |
| docs/guide/cli.md | 346 | CLI usage and install | current | L1 `# 명령줄 사용법`; git 2026-11-11 |
| docs/guide/hooks-and-skills.md | 75 | hook list and registration | current | L1 `# 훅과 스킬`; git 2026-11-02 |
| docs/guide/index.md | 34 | user guide index | current | L1 `# chaude 사용자 가이드`; git 2026-11-02 |
| docs/guide/models-and-agents.md | 128 | model split and agent table | current | L7 states Opus main; git 2026-11-09 |
| docs/guide/repl.md | 248 | REPL usage | current | L1 `# REPL 사용법`; git 2026-11-11 |
| notes/DECISIONS.md | 61 | decisions and style rules | current | L1 `# Decisions & Style Rules`; L12-14 cite Go paths (internal/harness); git 2026-11-11 |
| notes/FACTS.md | 11 | confirmed facts (API pricing) | current | L1 `# Confirmed Facts`; L6 pricing source, 확인일 2026-10-29; git 2026-10-31 |
| notes/LEDGER.md | 300 | running work log | current | L100-138 dated entries; tracked; git 2026-11-01 |
| notes/LESSONS.md | 6 | lessons | current | L1 `# LESSONS`; git 2026-11-05 |
| notes/OPEN.md | 9 | open items | current | L1 `# Open items`; L6 release history (0.11.3, deprecated/ deleted); git 2026-11-11 |
| notes/research/billing.md | 18 | Claude Code billing source check | current | L1 `(조회일 2026-10-29)`; git 2026-10-31 |
| notes/research/chaude-anthropic-terms.md | 61 | chaude vs Anthropic terms | current | L3 `fetched: 2026-11-07`; untracked (no git date) |
| notes/research/prompt-cache-ttl-clear.md | 56 | prompt cache TTL and /clear | current | L1 title; L39 Inferences section; git 2026-10-31 |
| record/LEDGER.md | 300 | agent run log | current | L1-2 dated agent entries; no git history (untracked) |

Status counts (34 files): current 22, done 3, superseded 4, stale 2, planned-only 3.

### Overlaps and contradictions

- Overlap: plan/gui-basic.md (superseded) and plan/gui-parity.md (current) both cover the GUI design.
- Overlap: plan/rust-engine-tui.md and plan/rust-port.md both give the Rust engine/TUI design and the same phase order (E1, T1, E2, T2, E3).
- Overlap: plan/rust-parity-checklist.md, plan/tui-parity.md and plan/rust-test-parity.md all track Go-to-Rust parity.
- Overlap: plan/rust-test-parity-p1/p2/p3.md duplicate plan/rust-test-parity.md (each says "Merged into").
- Overlap: docs/claude-lean/FRESH_START.md and docs/claude-lean/MIGRATE_NOW_PROMPT.md both cover resetting a large session.
- Overlap: install steps appear in docs/claude-lean/APPLY_GUIDE.md, docs/claude-lean/README.md (설치) and docs/guide/cli.md (설치).
- Overlap: docs/long-session-bench-plan.md (planned) and docs/pingpong-bench.md (results) cover the same efficiency test.
- Overlap: plan/caveman-structure.md and docs/claude-lean/CAVEMAN_FORMAT.md (canonical) both define the caveman format; plan/interactive-child-resident-sessions.md covers the same resident-session topic that caveman-structure marks SUPERSEDED.
- Overlap: notes/FACTS.md (API pricing) and notes/research/billing.md both cover pricing sources.
- Contradiction: plan/tui-parity.md L5 says all items unchecked; body has 181 `[x]` and 11 `[ ]`.
- Contradiction: plan/rust-engine-tui.md L19 and plan/rust-port.md phase 4 say inline output, no alternate screen; crates/cli/src/tui/mod.rs:1 and :43 use the alternate screen.
- Contradiction: plan/rust-port.md L16 and L48-52 say GUI is Tauri; plan/gui-parity.md L5-6 says eframe/egui (replaces Tauri).
- Contradiction: docs/claude-lean/CHANGELOG.md newest entry 0.5.3 vs release 0.12.5.
- Contradiction: plan/tui-parity.md and plan/rust-test-parity*.md cite `deprecated/` Go sources; deprecated/ is absent (notes/OPEN.md L6 "deprecated/ deleted"); last git commit touching it is 5de4d12 (2026-11-06).
- Contradiction: notes/DECISIONS.md L12-14 cite internal/harness (Go) paths that no longer exist.
- Resolved 2026-11-12: plan/caveman-structure.md no longer says design v7; the old text is in plan/archive/caveman-structure-history.md. Conflicts now listed in that file under "Known conflicts".

### Applied doc layout (2026-11-12)

Applied by the docs consolidation step. Table rows above show pre-move paths. Nothing was deleted.

- plan/rust-engine-tui.md -> moved (git mv) to plan/archive/rust-engine-tui.md; archive line added (superseded by plan/rust-port.md). Reference in plan/rust-port.md updated.
- plan/gui-basic.md -> moved (git mv, uncommitted edit included) to plan/archive/gui-basic.md; archive line added (superseded by plan/gui-parity.md). Reference in plan/gui-parity.md updated.
- plan/gui-parity.md -> kept; committed (was untracked).
- plan/rust-port.md -> Tauri wording changed to eframe/egui (L3, L16, L48, L52); inline/no-alt-screen wording annotated with the code note (crates/cli/src/tui/mod.rs:1); L53 deprecated/ marked historical.
- plan/tui-parity.md -> L5 counts set to 181 checked, 11 unchecked; L3 source marked historical.
- plan/rust-test-parity.md -> heading marked historical (deprecated/ deleted in 0.11.3).
- plan/rust-test-parity-p1.md, plan/rust-test-parity-p2.md, plan/rust-test-parity-p3.md -> NOT moved: crates/core/tests/parity_p12_harness.rs:1 and parity_harness_p3*.rs:1 cite them in code comments. First line "> Source of truth: plan/rust-test-parity.md" added.
- plan/rust-parity-checklist.md -> kept; first line marks deprecated/ paths historical (Go source at tag v0.11.2).
- notes/DECISIONS.md L12-14 -> "(historical, Go code removed)" appended; text not rewritten.
- plan/caveman-structure.md -> rewritten as the current design only (58 lines; sections: Scope, Channels, Brief format, Return formats, Guard hook, Known conflicts, Proposed). Old working-tree text (339 lines with header line) moved, not deleted: plan/archive/caveman-structure-history.md. Former section 15 (session load, screen refresh) -> end of plan/rust-port.md. Former section 16 (session count, usage) -> notes/research/subagent-usage.md.
- Not done, user confirmation needed: merge of docs/claude-lean/FRESH_START.md and MIGRATE_NOW_PROMPT.md; delete of archived files; status of docs/claude-lean/CHANGELOG.md.
- Not changed: notes/FACTS.md, notes/OPEN.md, CHANGELOG.md, README*.md, CLAUDE.md, record/.

## 1a. User field experience (2026-11-12)

- User report (2026-11-12), compared with plain Opus 5.5: usage difference is large, quality loss is almost none, speed is only slightly slower. User is satisfied.
- Evidence level: user impression. No bench or log was run for it. Quality side is unmeasured (see V(b)).
- Consequences:
  - No Opus-only comparison bench is planned.
  - S12 (main read access / conditional delegation) is deferred until a problem is felt.
  - The operating model itself stays.

## 2. Findings

### A. Release and CI
- 55 release tags in the 14 days before 2026-11-12 (63 total); 0.12.0 to 0.12.5 shipped within two days.
- `release.yml` (tag push) builds `chaude-cli` only; it has no test or clippy step. Tests before release run only through `release-dispatch.yml`.
- `update`, `clean`, `scripts` crates are outside push/PR CI (workspace default-members = core, cli, gui).
- Non-test `unwrap()`: update 43, clean 30. `update` is the installer/updater path.
### B. Tests
- Timing-dependent cli tests, the likely source of the two flaky failures in the G1a gate (not confirmed): `a_key_press_wakes_the_loop_without_waiting_for_the_tick`, `a_key_press_reaches_the_screen_without_waiting_for_the_tick` in `crates/cli/src/tui/mod.rs` (thread::sleep 20/200 ms + Instant).
- `crates/cli/src/tui/lang.rs:114` sets a process env var in a test; tests run in parallel.
- `crates/core/tests/engine.rs` (5,084 lines, 137 tests) depends on fake-claude scripts with sleep 0.2 to 2 s.
### C. Structure: UI state lives in the TUI crate
- `crates/cli/src/tui/mod.rs` is 8,457 lines; `App` has 72 fields; `apply` 523 lines, `on_key` 324 lines, main loop 208 lines with 31 `Wake` arms; terminal guard, signal forwarding, 5 thread spawns and clipboard process spawn live in the same file.
- `chaude-gui` depends only on `chaude-core` and is 29 lines; cli is 17,268 lines. None of the App logic is reusable by the GUI today.
### D. Structure: engine in one file
- `crates/core/src/api.rs` is 6,973 lines, 241 non-test fns, 19 pub fns; actor loop 290 lines, `on_event` 223 lines. Public surface is narrow (good); internals are not split.
- `proc.rs` (spawn) and `event.rs` (stream-json parse) are already separate; core has no terminal-specific code.
### E. Structure: rules duplicated between Python hooks and Rust
- 12 hook scripts exist twice (`.claude/hooks/` and `crates/core/src/settings/hooks/`, embedded with include_bytes!); byte-identical today, nothing enforces it.
- Language precedence differs: `_lean.py` = settings `language` then `CHAUDE_LANG`; `i18n.rs` = `CHAUDE_LANG` > settings > OS locale.
- Hooks and Rust share files by convention only (e.g. `record/metrics/<date>.jsonl`); no format check found.
### F. Structure: on-disk state
- State is spread over `<cwd>/.chaude/`, `notes/tmp/`, `record/`, `.claude/`, and the per-OS state dir (`~/.claude-harness` on mac).
- Only `handoff.rs` carries a version field. `edit.rs` uses a hand-rolled JSON codec for `history.jsonl` although serde_json is a dependency.
### G. Structure: errors and shared helpers
- `Result<_, String>`: core 28, cli 16. One custom error type in core (`lean.rs` ParseError).
- `home_dir` logic exists in core `osenv.rs`, cli `sessview.rs:190`, scripts `common.rs:465`.
### H. Harness weight and docs
- Rust code 51,368 lines vs `notes/` 1,722 files / 173,952 lines and `record/` 311 files; 12 hook scripts (1,272 lines), 13 agents.
- Main session has no read access, so design decisions rest on agent summaries.
- `CLAUDE.md` assumes Opus main; README agent table omits `haiku-test-lead`.
### I. Open items carried over
- haiku-prompter loop: launched a sub-agent in background and returned early; cannot reuse a coder across rounds; has no Write tool for logs.
- Release step blocked once by the permission classifier; mac install broke twice on GitHub CDN TLS resets (no retry/fallback).
- Token display: turn line empty after `/resume`; no recorded real stream fixture; `/resume` session selection to check.
- Generated settings lack the lang hook wiring; doctor detection of versions below 0.12.0 untested; stale "not wired" headers in `tui/agentview.rs` and `tui/rewind.rs`.

### J. Structure: logic duplicated between cli and core
- Session listing exists twice: `crates/cli/src/tui/sessview.rs:123 list_sessions` and `crates/core/src/resume_group.rs:237 list_sessions`; `project_dir` twice (`sessview.rs:182`, `core/src/record_folder.rs:42`).
- `crates/cli/src/tui/records.rs` has 12 fns (parse_header, scan_headers, unescape, parse_entries, is_legacy, sorted_entries, migrate_root, list_files, list_projects, load_project, filter_projects, pick) with name-matched counterparts in core `record.rs`, `record_folder.rs`, `record_turn.rs` (bodies not diffed).
- cli-only logic a second front end needs: input history (`tui/edit.rs`), startup record pruning (`cli/src/main.rs:172-173`).
- Hypothesis, not verified: the `/resume` wrong-session observation comes from the two listing implementations disagreeing.

### K. Structure: engine state is an implicit state machine
- `api.rs` `State` has 97 fields; `UiEvent` 48 variants; `Cmd` 11; `Input` 5.
- "What the engine is doing" is spread over `busy`, `hold_notice_turn`, `proc: Option`, `turn_started: Option`, `opus_agents`, `pending_end`, `usage_poll_at`; invalid combinations are not excluded by types. Largest match: `on_event` 220 lines (2839-3059).

### L. Structure: process and thread lifetime
- `ClaudeProc` has no `Drop`; a dropped value leaves its `claude` child running (`proc.rs:283-297` kills only in `finish`/`kill`).
- `EngineHandle::drop` (`api.rs:717-721`) sends `Cmd::Shutdown` without joining; child kill and exit handoff happen at actor exit (`api.rs:1559-1586`), so they may not complete if the process exits first. Explicit `shutdown()` (`api.rs:709-714`) joins.
- stdout reader threads (`proc.rs:215-227`) and MCP connection threads (`mcp.rs:161`, `:322`) are not joined. Actor panic path not checked.
- `ClaudeProc::finish` polls every 20 ms up to 5 s on the actor thread (`proc.rs:290`, via `api.rs:2449 exit_idle_coder`); other commands wait meanwhile. All three engine channels are unbounded (`api.rs:748-750`).

### M. Repo leftovers
- `tests/e2e-rebuild` holds Go files; `tests/chaude-soak` is a separate workspace with a 0.11.9 snapshot; 11 `core/tests/parity_*` files cite removed Go tests in header comments.

### N. Hook: correction-guard.py (`.claude/hooks/`, copy in `crates/core/src/settings/hooks/`)
- Korean regex, first hit wins. Class A has bare `말고` and sentence-final `…라고`; both give false positives on plain instructions.
- Does not separate real user prompts from task-notification or wrapper text.
- Counter file `logs/.corr-{sid}` never resets; this gives a false `[corr x2] stop`.
- Fix: skip notification and wrapper text; anchor broad patterns to sentence start; reset the counter on a non-correction prompt.
### O. Hook: style-adapter.py `[paste]` (`.claude/hooks/`, copy in `crates/core/src/settings/hooks/`)
- Triggers on len > 1500, > 20 newlines, or `<pasted_content`.
- Saves wrapped prompts and notifications. `record/pastes`: 277 files, 1,410,478 B; 45 contain `[original]`.
- Fix: measure only the text after `[original]`; skip notifications.
### P. Prompter summary is stateless
- Repeated the same resolved "unclear" item for 5 turns (observed, not verified in code).
- Fix: pass the last 1-2 turn conclusions, or skip the summary for short follow-ups.
### Q. Hook: context-alert.py is inert; two ledgers
- `LEAN_RESET_AT` = "0", so the hook does nothing, but it runs on every prompt (`.claude/hooks/`, copy in `crates/core/src/settings/hooks/`).
- Two ledgers: `record/LEDGER.md` (92 KB) and `notes/LEDGER.md` (83 KB).
- Fix: unregister the hook when the limit is 0; keep one ledger.
### R. No retry on API overload or rate limit
- No 429, overloaded, or backoff retry in `crates/core/src/`. Child death surfaces as "coder exited".
- Fix: classify API errors; bounded backoff retry. Needs user decision.
### S. Coder timeout drops partial work
- `CODER_TIMEOUT` is fixed at 30 min (`crates/core/src/resident.rs:13`). On expiry the child is dropped and only "coder timed out" is returned; partial work is lost.
- Fix: return partial output and changed files on timeout; make the timeout configurable.
### U. Caveman channel: return formats conflict (plan/caveman-structure.md "Known conflicts")
- U-a: R-write shape differs: CLAUDE.md:11 `changed: <path>: <what>` vs CAVEMAN_FORMAT.md 3.3 `changed: <path list>`; `changed` wording differs per agent file; hook REQUIRED omits summary_ko that 3.3 lists; prompter relay return has no changed/diffstat/check/ws.
- U-b: `relay` is read by crates/core/src/resident.rs:56 and named in CLAUDE.md:5, but not defined in CAVEMAN_FORMAT.md or haiku-output.md; prompter.md:38 uses the word for its own call.
- U-c: summary_ko overlaps the haiku-output Korean translation; language is English in opus-architect.md:45, unspecified in 3.3.
- U-d: orch.rs:32 RULE_B says "no relay hop", but the prompter hop and the haiku-output hop are both in the flow.
- U-e: haiku-test-lead and haiku-extractor are outside hook TARGETS and use their own formats; extractor.md:23 and :25 contradict each other.

### T. Haiku pool worker map is unbounded
- `crates/core/src/haiku_pool.rs` worker map has no cap, TTL, or eviction.
- Fix: cap the map and add an idle TTL.

### V. Operating model as documented (docs-only review)
- (a) Savings evidence is weak.
  - Short bench: cost -42.7% but total tokens +73.0%; n=2 per setting; the same setting differs by about 15% between runs (docs/pingpong-bench.md:39-40, :84).
  - README 69.9% comes from ratio R=1.73 of a 3-prompt run, applied to 229 sessions. The Opus-only baseline is computed, not measured (README.md:241-263).
  - -80% is labelled a prediction (docs/claude-lean/README.md:70-75).
- (b) Quality side is unmeasured. The decider sees only Haiku summaries; verification is also Haiku; notes/LESSONS.md has no Haiku-error entries.
- (c) Fixed per-turn stages: 3 Claude processes per turn (plan/rust-port.md:7). The one-round rule fits investigate-then-decide tasks poorly (docs/guide/models-and-agents.md:57).
- (d) Cache and rotation premises are unverified.
  - Thresholds differ across docs: 150k, 200k, 80k, 100k, 400k. Auto-compact 150k is below rotate 200k (Opus) (docs/claude-lean/README.md:44; plan/rust-port.md:66; docs/guide/cli.md:99).
  - Idle 3570 s assumes the 1h TTL (docs/guide/cli.md:120; notes/DECISIONS.md:58). plan/interactive-child-resident-sessions.md:26 and :55 suspect 5m for -p children.
  - "handoff cost net ~0" is computed; inputs not verified.
- (e) Doc contradictions.
  - Final-reply owner: docs/guide/models-and-agents.md:57 (main writes the answer) vs :52 (haiku-output sends the answer).
  - Agent count: 11 vs 12 (locations to confirm; .claude/agents has 13 files).
  - Main-model rule: notes/DECISIONS.md:40 (Opus main only for design turns) vs :50 (default main = opus). Status of :40 to confirm.
  - Name haiku-prompter is used for two things: the loop agent (README.md:50) and the per-input summary hop (Finding P, S26).

## 3. Steps

Order = dependency order. Each step: own commit, gate through haiku-test-lead, TUI behaviour unchanged unless stated.
- [ ] S1 CI safety: add test + clippy to `release.yml` before build; run `cargo test --workspace` on push/PR so update/clean/scripts are covered. Accept: a tag push with a failing test produces no release asset.
- [ ] S2 Flaky tests: run the two key-press tests 200 times; replace sleeps with a deterministic wake signal; serialize or remove the env-var test in `lang.rs`. Accept: 200/200 passes, full cli suite passes 5 times in a row.
- [ ] S3 Loop agent: haiku-prompter runs sub-agents foreground, may continue a coder via SendMessage, may write its log. Accept: one 3-round dry loop with a single coder ID and a written log.
- [ ] S4 App state to core: move `App` state, `Action`, and `apply` into a UI-neutral core module; TUI keeps key mapping, rendering, terminal and signals. Accept: existing TUI tests pass unchanged; gui can construct the state and apply an action in a test. This replaces the start of G1b in `plan/gui-parity.md`.
- [ ] S5 Split `api.rs` into modules (actor, event handling, agents, tokens) with no pub API change. Accept: `cargo public-api`-style diff of pub items is empty (or manual list equal), tests pass.
- [x] S6 Single hook source: `crates/core/src/settings/hooks/` is the source, `.claude/hooks/` is generated; add a test that fails when they differ. Decide one language precedence (proposed: `/lang` setting > `CHAUDE_LANG` > OS locale) and implement it in both `_lean.py` and `i18n.rs`.
- [ ] S7 State files: add a `version` field to each persisted file, read old files without it; replace the hand-rolled history codec with serde_json. Accept: files written by 0.12.5 load in the new build (fixture test).
- [ ] S8 Errors: one core error enum; replace `Result<_, String>` module by module; remove non-test `unwrap()` in update and clean; one shared `home_dir`.
- [ ] S9 Release path: retry with backoff plus CI-artifact fallback for the mac install; decide allowlisting of the release step.
- [ ] S10 Docs: apply section 1 "Proposed doc layout"; fix CLAUDE.md main-model wording, README agent table, `plan/tui-parity.md` header, agentview/rewind headers. Partly done 2026-11-12: doc-layout moves and tui-parity/rust-port status fixes applied (section 1, "Applied doc layout"). Still open: CLAUDE.md main-model wording, README agent table, agentview/rewind headers.
- [ ] S11 Proposed scope for 0.13.0: S1 to S4 plus GUI output pane, input box, status bar; remaining GUI features in 0.13.x. Needs user decision (current order is "all TUI features in 0.13.0").
- [ ] S12 Needs user decision: read-only (Read/Grep) access for the main session.
- [ ] S13 One implementation for sessions and records: cli `sessview.rs`/`records.rs` call core; delete the cli copies after diffing behaviour; move input history and record pruning to core. Do with S4. Accept: a test lists the same sessions through the single core fn that both `/resume` and the session view use.
- [ ] S14 Process lifetime: `Drop` for `ClaudeProc` (kill + wait); `EngineHandle::drop` joins with a timeout; actor wrapped so a panic still kills children; non-blocking coder exit (no 5 s poll on the actor thread). Accept: test that drops the handle and finds no live child pid; Interrupt is handled within 100 ms while a coder is exiting.
- [ ] S15 Engine phase enum: replace the busy/hold/proc/turn_started flag set with one `Phase` enum plus per-phase data; do during S5. Accept: no pub API change, tests pass.
- [ ] S16 Remove or archive `tests/e2e-rebuild` (Go) and decide on `tests/chaude-soak`; update parity test header comments. Needs user decision for deletion.
- [x] S17 Correction guard: skip notification and wrapper text; anchor broad patterns (`말고`, `…라고`) to sentence start; reset `logs/.corr-{sid}` on a non-correction prompt; change both hook copies. Accept: plain instructions containing `말고` or `…라고` and a notification text give no hit in a test set; the counter returns to 0 after one non-correction prompt.
- [x] S18 Paste hook: measure only the text after `[original]`; skip notifications in `[paste]`; change both hook copies. Accept: re-run the `record/pastes` scan; no new saved file is a notification; the size check uses only the text after `[original]`.
- [ ] S19 Prompter summary: confirm the repeated "unclear" item in code or logs first (not verified); then pass the last 1-2 turn conclusions, or skip the summary for short follow-ups. Accept: a 5-turn dry run does not repeat the same resolved item.
- [x] S20 Context alert and ledgers: unregister `context-alert.py` when `LEAN_RESET_AT` is 0; merge `record/LEDGER.md` and `notes/LEDGER.md` into one ledger. Which ledger to keep: user confirmation needed. Accept: with limit 0 the hook is not in settings; one ledger file is left and every writer uses it.
- [ ] S21 API retry: classify 429, overloaded, and child-death errors; retry with bounded backoff; show "coder exited" only after retries end. Needs user decision (retry count, max wait, whether auto-retry is allowed). Accept: a fake-claude test that returns 429 twice and then succeeds completes the turn; a permanent error shows its classified message.
- [ ] S22 Coder timeout: on `CODER_TIMEOUT`, return partial output and the changed-file list; make the timeout configurable. Accept: an engine test where the coder times out returns partial output and changed files; the timeout value is read from config in a test.
- [ ] S23 Haiku pool: cap the worker map and add an idle TTL; evict and stop idle workers. Accept: after cap+1 workers and the idle TTL, the map size stays at the cap and no idle worker child is left alive.
- [ ] S24 Caveman return shape: one shape (status / reason / body / relay) defined only in docs/claude-lean/CAVEMAN_FORMAT.md; per-agent R-* blocks become body types; apply to the 13 agents in .claude/agents, the embedded copies in crates/core/src/settings/agents, and the hook. Drop summary_ko only if user decides so (user confirmation needed). Accept: `grep -rn "changed: <" .claude/agents crates/core/src/settings/agents` shows one wording; the hook REQUIRED list matches CAVEMAN_FORMAT.md; `cargo test -p chaude-core` passes for the embedded agent copies.
- [ ] S25 Caveman guard hook: cover all 13 agents (TARGETS in .claude/hooks/caveman-format.py), or remove the hook. Mode (warn vs remove) needs user decision. Accept: each of the 13 agent names is in TARGETS or the hook is absent from .claude/settings.json; a fake agent output with a missing R-write field yields a row (and additionalContext when CAVEMAN_GUARD=warn).
- [ ] S26 Hop-skip policy: direction chosen by user 2026-11-12 = skip the prompter summary for short follow-up inputs (ack words, one-line instructions). Accept: such inputs reach main without summary; measured latency drop.
- [ ] S27 Measure real cache TTL of child processes once. Accept: observed cache_read vs cache_creation after 6 min and after 50 min idle; notes/DECISIONS.md updated.
- [ ] S28 One context-threshold table as the single source (150k, 200k, 80k, 100k, 400k); other docs point to it. Accept: each threshold value appears only in that table; other docs link to it.
- [x] S29 Fix stale section refs to the old caveman-structure numbering in CLAUDE.md:11 and docs/claude-lean/CAVEMAN_FORMAT.md:3, :32. Accept: each ref points to a section that exists in plan/caveman-structure.md.
- [ ] S30 Resolve the doc contradictions in V(e). Final-reply owner and main-model rule need user confirmation. Accept: one final-reply owner, one agent count, one main-model rule across docs.

## 3a. Execution order

Arranges the existing steps S1-S30 into phases. Step text in section 3 is not changed.
Run phases in order. Each phase exit criterion must pass before the next phase starts.
Every step ID appears once in the phase, Deferred or Unplaced lists below. "Decided 2026-11-12" repeats IDs as a cross-reference.

### P1. Per-prompt hooks
- Reason: smallest change; these hooks run on every prompt.
- Order:
  - S17 correction guard false positives
  - S18 paste hook measures wrong text
  - S20 inert context alert, two ledgers
  - S19 prompter summary stateless
  - S26 skip summary on short follow-ups
- Exit: S17 and S18 accept tests pass on both hook copies; S20 hook absent when limit is 0; S19 and S26 accept checks pass.

### P2. Single source of truth and one measurement
- Reason: thresholds, docs and hook copies drift now; one measurement feeds the rotation and TTL decisions.
- Order:
  - S6 single hook source, language order
  - S27 measure real cache TTL
  - S28 one context-threshold table
  - S29 fix stale caveman section refs
  - S30 resolve doc contradictions
  - S10 docs layout and headers
  - S24 one caveman return shape
  - S25 guard hook covers all agents
- Exit: hook copies identical by test (S6); S27 result in notes/DECISIONS.md; each threshold value in one table only (S28); one final-reply owner, agent count and main-model rule (S30); REQUIRED list matches CAVEMAN_FORMAT.md (S24); all 13 agents in TARGETS or hook removed (S25).

### P3. GUI prerequisite, then GUI
- Reason: the GUI cannot reuse App logic or session listing until core owns them.
- Order:
  - S11 0.13.0 scope decision (start of phase)
  - S4 move App state and apply to core
  - S13 one session and records implementation
  - G1 to G4 GUI steps per plan/gui-parity.md (not S-IDs; G0 status: user confirmation needed)
- Exit: S4 and S13 accept tests pass; a gui test constructs state and applies an action; S11 scope recorded before GUI work; G3 parity table has no gaps.

### P4. Engine robustness
- Reason: engine behaviour changes; split and state type first so the fixes land in smaller modules.
- Order:
  - S5 split api.rs into modules
  - S15 engine Phase enum
  - S14 process lifetime and drop
  - S21 API error retry with backoff
  - S22 coder timeout keeps partial work
  - S23 haiku pool cap and TTL
- Exit: S5 pub item diff empty; S15 tests pass with no pub API change; S14 drop test leaves no live child; S21 fake 429 twice then success completes the turn; S22 timeout test returns partial output; S23 cap+1 test keeps map at cap.

### P5. Hygiene
- Reason: no user-facing change; safe after behaviour settles.
- Order:
  - S1 CI test and clippy gate
  - S2 flaky key-press and env tests
  - S9 release retry and fallback
  - S7 versioned state files
  - S8 core error enum, unwrap cleanup
  - S16 remove or archive e2e-rebuild
- Exit: S1 failing test blocks release asset; S2 200/200 key-press runs and 5 clean cli runs; S7 0.12.5 fixture loads; S8 no non-test unwrap in update or clean; S16 deletion decided.

### Deferred
- S12 read-only access for main session. Deferred per section 1a until a problem is felt.

### Unplaced
- S3 haiku-prompter loop (foreground sub-agents, coder reuse, log write). Fits no phase: it is an agent-loop change, not a hook, GUI, engine or hygiene item. Not forced into a phase; can join P4 if user decides.

### Decided 2026-11-12
- S11 (P3): all TUI features in 0.13.0 (current order kept).
- S6 (P2): language precedence `/lang` > `CHAUDE_LANG` > OS locale.
- S24 (P2): drop summary_ko; relay only.
- S25 (P2): warn mode; cover all 13 agents in TARGETS.
- S30 (P2): final reply owner = haiku-output; main model = Opus by default; Haiku main only with `--model haiku`.
- S20 (P1): keep record/LEDGER.md as the ledger.
- S21 (P4): auto-retry max 3 times with backoff; total wait <= 60 s.
- S9 (P5): retry with backoff plus CI-artifact fallback; no allowlist.
- S16 (P5): delete tests/e2e-rebuild; keep tests/chaude-soak.
- S26: skip the prompter summary for pure ack words only.
- S19: one paid 5-turn dry run allowed.
- S12 (Deferred): not decided on 2026-11-12; stays deferred.

## 4. Not verified

- No build or test was run; flake cause and rate are unmeasured.
- Non-test counts are heuristic; cfg counts include test code.
- Duplication between update/clean/scripts beyond `home_dir` not confirmed.
- Thread/sync primitive counts in core not measured.
- cli/core record fn bodies not diffed; update/clean/scripts vs core duplication, install overwrite behaviour, python3 runtime check, version sync mechanism not checked.
- Not read: the six engine flows that make model calls per trivial prompt (call count and cost not measured).
- Not read: turn-end detection and timers.
- Not read: context rotation and handoff cost.
- Not read: cache keep-alive.
- Not read: `mode::decide` rules (`crates/core/src/mode.rs:71`).
- Not verified: possible token double-counting (`crates/core/src/turn_spend.rs:26-120`).
