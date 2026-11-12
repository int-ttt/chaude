# Caveman structure: current design

status: current design, as implemented 2026-11-12. Decision history: plan/archive/caveman-structure-history.md.
sources: docs/claude-lean/CAVEMAN_FORMAT.md (canonical formats), CLAUDE.md, .claude/agents/*.md (13), .claude/hooks/caveman-format.py.

## Scope
- Formats for briefs and returns between Opus main, Haiku main, and agents.
- The canonical field definitions are in docs/claude-lean/CAVEMAN_FORMAT.md. This file records what is implemented and where it differs.

## Channels
- L0: user-facing text. Only `haiku-output` writes it, in the CHAUDE_LANG language (`ko` default, `en`). CLAUDE.md:10.
- L1: main <-> `opus-architect`. Haiku main path only: user prompt verbatim plus summary, then R-decide back. CLAUDE.md:5.
- L2: Opus main <-> `haiku-*` agents. Briefs in English caveman, returns in English caveman. CLAUDE.md:10.
- Caveman lines are never shown to the user. Returns go through `haiku-output` (CLAUDE.md:11).

## Brief format (B)
- Keys: `task`, `paths`, `ctx`, `limits`, `return`, `lang`. CAVEMAN_FORMAT.md 3.1.
- `lang: caveman | ko | en`. `ko` requires summary_ko in the return.
- Agents' own briefs are at most 5 lines (CLAUDE.md:16).
- Hook checks B keys only for `haiku-prompter` (see Guard hook).

## Return formats (as implemented)
- Head (all R-*): `status: done | partial | fail | blocked` and `reason:`. CAVEMAN_FORMAT.md 3.6. Used by all agents with an R block.
- R-read (`haiku-scout`, `haiku-researcher`): claim, searched, not_found, not_checked, truncated, omitted, summary_ko (only lang ko). scout.md:24-34, researcher.md:21-31.
- R-write (`haiku-coder`, `haiku-drafter`, `haiku-batch-worker`, `haiku-checker`, `haiku-runner`, `sonnet-writer`): changed, diffstat, check, ws, summary_ko (only lang ko). Field wording differs per file (see (a)).
- R-decide (`opus-architect`): decision, why, rejected, risks, open_q, plan_path, summary_ko (English, lang ko). Matches CAVEMAN_FORMAT.md 3.4.
- relay return (`haiku-prompter`, relay calls to opus): status, reason, relay, rounds, answer (verbatim), truncated, omitted. No summary_ko. prompter.md:48-56.
- `haiku-output`: no R block. Input is R-* blocks or summary_ko. Output is natural text in the CHAUDE_LANG language. haiku-output.md:11-14.
- `haiku-test-lead`: one PASS/FAIL table, at most 10 lines, no 3.6 head. test-lead.md:42.
- `haiku-extractor`: at most 3 lines (path, item counts). extractor.md:23 also says "3.6 header + R-* block" (line 25). Self-contradictory.
- Embedded copies in crates/core/src/settings/agents/*.md: key lines match .claude/agents for the R-write and R-decide files checked (not checked for every file).

## Guard hook (.claude/hooks/caveman-format.py)
- Event: PostToolUse, matcher `Agent|Task` (.claude/settings.json:107-111, working tree).
- Never blocks: always exit 0.
- `CAVEMAN_GUARD`: `log` (default, writes a metrics row only), `warn` (row + additionalContext), `off`.
- Rows go to record/metrics/<date>.jsonl (`HARNESS_RECORD_DIR` overrides the dir).
- Targets: 11 agents (TARGETS, line 14): scout, researcher, coder, batch-worker, runner, checker, drafter, sonnet-writer, opus-architect, tui, prompter.
- Not targets: `haiku-test-lead`, `haiku-extractor`. 11 of 13 agent files.
- Checks when the brief has `return: R-*`: status and reason present; status value valid; R-read: claim, searched, not_found, not_checked, truncated (yes|no), claim as `path:line |`; R-write: changed, diffstat, check, ws; R-decide: decision, why, rejected, risks, open_q, plan_path; summary_ko when prompt has `lang: ko`.
- `haiku-output`: polite-form check and leak check (caveman keys in output).
- `haiku-prompter`: B keys check only.
- Legacy brief without `return:` line is not checked.
- Metrics row writing in practice: unverified (not checked this run).

## Known conflicts
- (a) R-write shape differs in 4 places: (1) CLAUDE.md:11 `changed: <path>: <what>` vs CAVEMAN_FORMAT.md 3.3 `changed: <path list>`; (2) `changed` wording `<list of paths>` (coder, drafter, batch-worker, sonnet-writer) vs `<path list>` (checker, runner); (3) hook REQUIRED omits summary_ko, 3.3 lists it; (4) prompter relay return has relay/answer/rounds and no changed/diffstat/check/ws.
- (b) `relay` is read by crates/core/src/resident.rs:56 ("Use the relay field if present, else summary_ko") and named in CLAUDE.md:5. It is not defined in CAVEMAN_FORMAT.md or haiku-output.md. The word also names prompter's relay call (prompter.md:38).
- (c) summary_ko overlaps the haiku-output translation: 3.3 requires it for lang ko, and haiku-output renders Korean from the same content. Language of summary_ko: English in opus-architect.md:45, unspecified in 3.3.
- (d) "no relay hop" (orch.rs:32, RULE_B: "no relay hop") vs the prompter hop (before Opus main) and the haiku-output hop (after the final reply). Both hops are in the flow.
- (e) `haiku-test-lead` and `haiku-extractor` are outside hook targets and have their own formats (see Return formats).

## Proposed (not decided)
- One return shape: status / reason / body / relay, defined only in CAVEMAN_FORMAT.md. Per-agent R-* blocks become body types.
- Drop summary_ko if haiku-output can translate from body. user confirmation needed (사용자 확인 필요).
- Hook covers all 13 agents, and either warns (`CAVEMAN_GUARD=warn`) or is removed. user confirmation needed (사용자 확인 필요).
- Policy for skipping prompter and haiku-output hops when the user asks for a fast path. user confirmation needed (사용자 확인 필요).
- History: plan/archive/caveman-structure-history.md. Former section 15 is at the end of plan/rust-port.md. Former section 16 is at notes/research/subagent-usage.md.
