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
| plan/caveman-structure.md | 338 | caveman-based new structure design | planned-only | L3 `status: 설계안 v7 (구현 코드·측정 절차 미포함)`; L14 and L38 carry inline "SUPERSEDED 2611" notes; git 2026-11-01; uncommitted edit |
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
- Contradiction: plan/caveman-structure.md L3 says design v7 (planned), but L14 and L38 mark parts SUPERSEDED.

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
- Not done, user confirmation needed: merge of docs/claude-lean/FRESH_START.md and MIGRATE_NOW_PROMPT.md; delete of archived files; status of docs/claude-lean/CHANGELOG.md.
- Not changed: plan/caveman-structure.md (uncommitted edits kept, not staged), notes/FACTS.md, notes/OPEN.md, CHANGELOG.md, README*.md, CLAUDE.md, record/.

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

## 3. Steps

Order = dependency order. Each step: own commit, gate through haiku-test-lead, TUI behaviour unchanged unless stated.
- [ ] S1 CI safety: add test + clippy to `release.yml` before build; run `cargo test --workspace` on push/PR so update/clean/scripts are covered. Accept: a tag push with a failing test produces no release asset.
- [ ] S2 Flaky tests: run the two key-press tests 200 times; replace sleeps with a deterministic wake signal; serialize or remove the env-var test in `lang.rs`. Accept: 200/200 passes, full cli suite passes 5 times in a row.
- [ ] S3 Loop agent: haiku-prompter runs sub-agents foreground, may continue a coder via SendMessage, may write its log. Accept: one 3-round dry loop with a single coder ID and a written log.
- [ ] S4 App state to core: move `App` state, `Action`, and `apply` into a UI-neutral core module; TUI keeps key mapping, rendering, terminal and signals. Accept: existing TUI tests pass unchanged; gui can construct the state and apply an action in a test. This replaces the start of G1b in `plan/gui-parity.md`.
- [ ] S5 Split `api.rs` into modules (actor, event handling, agents, tokens) with no pub API change. Accept: `cargo public-api`-style diff of pub items is empty (or manual list equal), tests pass.
- [ ] S6 Single hook source: `crates/core/src/settings/hooks/` is the source, `.claude/hooks/` is generated; add a test that fails when they differ. Decide one language precedence (proposed: `/lang` setting > `CHAUDE_LANG` > OS locale) and implement it in both `_lean.py` and `i18n.rs`.
- [ ] S7 State files: add a `version` field to each persisted file, read old files without it; replace the hand-rolled history codec with serde_json. Accept: files written by 0.12.5 load in the new build (fixture test).
- [ ] S8 Errors: one core error enum; replace `Result<_, String>` module by module; remove non-test `unwrap()` in update and clean; one shared `home_dir`.
- [ ] S9 Release path: retry with backoff plus CI-artifact fallback for the mac install; decide allowlisting of the release step.
- [ ] S10 Docs: apply section 1 "Proposed doc layout"; fix CLAUDE.md main-model wording, README agent table, `plan/tui-parity.md` header, agentview/rewind headers. Partly done 2026-11-12: doc-layout moves and tui-parity/rust-port status fixes applied (section 1, "Applied doc layout"). Still open: CLAUDE.md main-model wording, README agent table, agentview/rewind headers.
- [ ] S11 Proposed scope for 0.13.0: S1 to S4 plus GUI output pane, input box, status bar; remaining GUI features in 0.13.x. Needs user decision (current order is "all TUI features in 0.13.0").
- [ ] S12 Needs user decision: read-only (Read/Grep) access for the main session.

## 4. Not verified

- No build or test was run; flake cause and rate are unmeasured.
- Non-test counts are heuristic; cfg counts include test code.
- Duplication between update/clean/scripts beyond `home_dir` not confirmed.
- Thread/sync primitive counts in core not measured.
