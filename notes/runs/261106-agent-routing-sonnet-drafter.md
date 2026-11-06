task: add sonnet-drafter (one-shot creative prose), narrow haiku-drafter, allow in hooks
status: done (no commit)
changed:
- .claude/agents/sonnet-drafter.md: new; haiku-drafter body, model sonnet, desc "Sonnet. One-shot creative..."
- .claude/agents/haiku-drafter.md: desc narrowed to simple/repeatedly revised prose; creative -> sonnet-drafter
- crates/core/src/settings/agents/{haiku-drafter,sonnet-drafter}.md: embedded copies, synced
- crates/core/src/settings/project.rs: AGENTS += sonnet-drafter.md (after opus-architect)
- crates/clean/src/lib.rs: PKG_FILES += .claude/agents/sonnet-drafter.md
- .claude/hooks/main-thread-guard.py (+ crates copy): MAIN_AGENTS += sonnet-drafter, docstring line
  (user's uncommitted 12-line guard edit kept; only added names)
- .claude/hooks/haiku-context-guard.py (+ crates copy): ALLOW_IO = ("haiku-", "sonnet-drafter")
  (else sonnet-drafter file tools denied as non-haiku subagent)
- .claude/hooks/agent-model-pin.py (+ crates copy): model strip also for sonnet-drafter
- .claude/hooks/caveman-format.py (+ crates copy): TARGETS += sonnet-drafter
- CLAUDE.md: line 4, 5, 6, 7 amended; 34 lines (unchanged)
checks:
- cargo test -p chaude-core -p chaude-clean -> PASS (243 core lib + 98 + 11 clean; embedded_agents_match_repo, embedded_hooks_match_repo, manifest_matches_repo ok)
- python3 ast.parse on 4 hooks -> PASS
- simulated hooks: main-thread-guard sonnet-drafter allow, general-purpose deny; agent-model-pin strips model for sonnet-drafter; haiku-context-guard: opus-architect Write deny, sonnet-drafter/haiku-coder allow
- cargo test -p chaude-cli not run (no agent/hook embeds there)
doubts:
- allow_large denial in haiku-context-guard now also applies to sonnet-drafter (shared branch)
- haiku-context-guard fail-closed regex still haiku-only (only on hook exception)
- docs (docs/guide/models-and-agents.md, docs/claude-lean/*, README*) not updated; out of brief
- agent-model-pin comment "sonnet tier removed 2026-10-31" now stale
- .claude/settings.json (user edit) untouched
follow-up (coordinator, user verbatim: sonnet-drafter only when user names it or story/game/important writing; else haiku-drafter):
- rule text applied: haiku-drafter desc (both copies) = default prose writer; sonnet-drafter desc (both copies) = explicit or story/game/important only; CLAUDE.md lines 5-6.
- agent-model-pin.py docstring "sonnet tier removed" comment replaced (both copies).
- cargo test -p chaude-core -p chaude-clean -> PASS (11, 243, 98, 1, 7, 2, 3, 11, 1, 9, 13, 3 passed; 0 failed)
- commit/push: NOT done. Reason: commit/push needs the user's own request; the coordinator message is not user consent. Also trailer conflict: system attribution says Co-Authored-By: Claude Haiku 5.5, message said Opus 5.5.
