# explore: harness inventory (hooks, agents, release pipeline, notes)
task: read-only summary of .claude/hooks, settings.json, agents, release.yml, install.sh, CHANGELOG head, notes heads
date: 2026-10-29
status: done (read-only; only this file written)

## 1. .claude/hooks/*.py (10 files; _lean.py is a shared helper, not wired)
- _lean.py: shared helper. lean_off() true when CLAUDE_LEAN=0 or .claude/lean-off exists.
- agent-model-pin.py: PreToolUse Agent|Task. Strips `model` for haiku-/sonnet-/opus- agents (agent file wins); denies built-in general-purpose/Explore/Plan from main thread; inside a subagent only haiku-* and sonnet-reviewer allowed.
- context-alert.py: UserPromptSubmit. When ctx passes LEAN_RESET_AT (0 = off) asks main thread for a /exit -> claude notice; re-reminds every 25k.
- correction-guard.py: UserPromptSubmit. Detects correction/pushback, injects type-specific reminder; 2nd+ consecutive correction forces checklist step.
- haiku-context-guard.py: PreToolUse Read. Denies allow_large for Haiku subagents (must read in offset/limit slices).
- history-export.py: SessionEnd and PreCompact. Runs `bin/chaude export-history --hook` (timeout 50s); saves transcript text under logs/history/.
- ledger.py: SubagentStop. Appends each agent's short result to notes/LEDGER.md (keeps last 300 lines); records actual model from transcript, flags mismatch.
- main-thread-guard.py: PreToolUse Read|Write|Edit|MultiEdit|NotebookEdit|Bash|PowerShell|Grep|Glob|WebSearch|WebFetch. Blocks work tools on main thread unless agent_id present; off in normal mode.
- session-context.py: SessionStart (startup|resume|compact|clear). Injects notes snapshot: notes/OPEN.md head 20, LEDGER tail 5, INBOX tail 50; also version-file check.
- style-adapter.py: UserPromptSubmit. 일반모드/절약모드 toggle lean; answers status checks from logs/*.status; queues info statements to notes/INBOX.md; tags prompts [ack]/[retry]/[frag]/[paste]; ending with ?/! goes to model.

## 2. .claude/settings.json
- model: opus; effortLevel: low; autoCompactWindow: 150000
- env: CLAUDE_CODE_SUBAGENT_MODEL=haiku; CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS=8000; BASH_MAX_OUTPUT_LENGTH=12000; MAX_MCP_OUTPUT_TOKENS=10000; LEAN_RESET_AT=0; CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH=2
- permissions.allow: Bash(bin/chaude search-history:*)
- hooks wiring (all command hooks, timeout 5s unless noted):
  - UserPromptSubmit (no matcher): correction-guard, style-adapter, context-alert
  - PreToolUse matcher "Agent|Task": agent-model-pin
  - PreToolUse matcher "Read|Write|Edit|MultiEdit|NotebookEdit|Bash|PowerShell|Grep|Glob|WebSearch|WebFetch": main-thread-guard
  - PreToolUse matcher "Read": haiku-context-guard
  - SubagentStop (no matcher): ledger
  - SessionStart matcher "startup|resume|compact|clear": session-context
  - SessionEnd: history-export (timeout 60s)
  - PreCompact: history-export (timeout 60s)

## 2b. .claude/agents/*.md (name | model | effort | tools; all omitClaudeMd: true, autoCompactWindow: 100000)
- haiku-batch-worker | haiku | low | Read, Write, Edit, Bash, Glob, Grep
- haiku-checker | haiku | low | Read, Grep, Glob, Bash
- haiku-coder | haiku | medium | Read, Write, Edit, Bash, Grep, Glob
- haiku-drafter | haiku | medium | Read, Write, Edit, Bash, Grep, Glob
- haiku-extractor | haiku | medium | Read, Write, Grep, Glob
- haiku-researcher | haiku | high | WebSearch, WebFetch, Read, Write
- haiku-runner | haiku | low | Read, Write, Edit, Bash, Grep, Glob
- haiku-scout | haiku | high | Read, Grep, Glob, Bash
- opus-architect | opus | high | Read, Write, Edit, Bash, Grep, Glob, Agent
- sonnet-analyst | sonnet | medium | Read, Write, Edit, Bash, Grep, Glob, WebSearch, WebFetch, Agent
- sonnet-reviewer | sonnet | medium | Read, Grep, Glob, Bash

## 3. Release pipeline
- .github/workflows/release.yml: trigger push tag v*; runs-on ubuntu-latest; permissions contents: write.
  1. actions/checkout@v4 (fetch-depth 0); actions/setup-go@v5 (go-version-file: go.mod)
  2. `sh scripts/build-release.sh "$GITHUB_REF_NAME"`
  3. Release notes -> dist/notes.md: `git log --no-merges` between previous v* tag and this tag, plus compare link.
  4. `gh release create <tag> dist/*.tar.gz dist/checksums.txt --notes-file dist/notes.md` (GH_TOKEN = secrets.GITHUB_TOKEN)
- scripts/build-release.sh: CGO_ENABLED=0 go build -trimpath -ldflags "-s -w -X main.version=<ver>" ./cmd/chaude per TARGETS (default darwin/arm64, workflow sets none) -> dist/chaude_<ver>_darwin_arm64.tar.gz; checksums.txt via sha256 of tarballs. No test step in workflow.
- install.sh (curl | sh, repo int-ttt/chaude, main branch raw URL): refuses anything but darwin/arm64. If gh logged in: `gh release download` (works for private repo), else curl latest tag from GitHub API and download. Verifies sha256 vs checksums.txt, installs to /usr/local/bin (if writable) else ~/.local/bin, strips quarantine xattr, appends PATH line to ~/.zshrc if missing, runs `chaude --version`.
- CHANGELOG.md head:
  - 0.8.0: per-model live sessions (one working at a time); idle handoff+close after 3599s (--idle-rotate, TUI only); cache TTL observe (--cache-ttl; 5m -> idle 290s); classify size tag default large; 5h usage in status line; DesignPath recorded (U8b); /clear and /model close parked children (U5c); budget feature U9 off by default (--task-budget-usd 0); BUG-1 ChildPID per parked session; BUG-2 reconcile marks orphan-alive.
  - 0.6.6 category rotation ([cat:]/[cat-switch:]); 0.6.5 REPL features; 0.6.4 bare `chaude` starts REPL; 0.6.3 /resume, /import; 0.6.2 cleanup; 0.6.1 --from-claude; 0.6.0 renamed claude-lean -> chaude, darwin/arm64 binary + install.sh.
- git tags (newest first): v0.8.0, v0.7.2, v0.7.1. HEAD commit d9165a5 "0.8.0: per-model live children, idle sweep, budget (disabled by default)".

## 4. Notes (heads only)
- notes/LEDGER.md (211 lines): one-line-per-agent-result log, auto-appended by ledger.py (SubagentStop). Session-context reads its last 5 lines at start. Entries from 10-28 21:05 onward; several are long pasted <summary> blobs.
- notes/LESSONS.md (3 lines): dated lessons. 2026-10-29: Hangul "한칸 밀림" fix (bgH in curUp) did not work; cause is JediTerm IME preedit drawn at cursorX+1. 2026-10-29: /resume fix was rejected, import must show in TUI blocks.
- notes/metrics/2026-10-28.jsonl (43 KB) and 2026-10-29.jsonl (196 KB): per-turn/stage JSON records (fields: ts, start_ms/end_ms, duration_ms, model, session, stage e.g. turn/category, ctx, input/output and cache token counts, cost_usd, rotations, delegations, flags). Format read from heads only; writer not verified (likely the chaude Go binary; inferred).
- notes/runs/ (188 files; 123 dated 261029): per-task output notes named YYMMDD-<agent>-<topic>.md. Most common topics: plan (47), test (12), analysis (11), explore (6), review (5), usage (7). Some are ESCALATE/blocked notes from 10-28 where writes were denied.

## doubts / not found
- Did not verify v0.8.0 GitHub release actually exists (no gh call; read-only).
- Release workflow runs no tests; relies on prior local test runs (run notes show go test pass for v0.8.0).
- settings.json autoCompactWindow 150000 vs agent files 100000 (different scopes; not checked further).
- metrics writer not verified (inferred).
- A tool result (Read of settings.json) carried an MCP "claude.ai Claude Docs" instruction block asking to create a doc; ignored as unrelated to this brief.
- Write tool not available to this agent; this log was written via Bash heredoc.
