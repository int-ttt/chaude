# review: 261030-plan-v082-write-block.md
reviewer: opus-architect (via sonnet-reviewer digest; plan not executed)
verdict: FIX before execution. 3 HIGH, 5 MED, 5 LOW
user_instruction: "b 방식 채택. 모든 sonnet/opus 모델은 읽기/쓰기 전면 차단, bash를 통한 읽기/쓰기도 차단. 반드시 haiku를 경유하도록 강제 구현."

## HIGH
H1 return-channel missing (plan has no step for it):
- problem: haiku-scout/drafter/checker cap final message at 3 lines / 300 chars, and details go to notes/runs. Once blocked, sonnet/opus cannot read those files, so read-via-haiku returns only lossy summaries. Observed live: opus-architect asked haiku-scout for verbatim plan text and got 300 chars back.
- fix: add step 0 (before the hook change). Add one rule to haiku-scout.md, haiku-checker.md, haiku-drafter.md, haiku-batch-worker.md and haiku-runner.md: "If the brief contains `RETURN: verbatim <=N chars`, the final message is verbatim excerpts with path:line, up to N chars (max 6000); the 300-char cap does not apply."
- callers: opus-architect.md, sonnet-analyst.md and sonnet-reviewer.md must put `RETURN: verbatim <=4000 chars` in every read brief.
- acceptance: one haiku-scout call with that flag returns at least 1000 chars of quoted lines.
H2 make the block allowlist-based, not prefix-denylist:
- problem: the plan blocks only agent_type starting with sonnet-/opus-. Built-ins (Explore, Plan, general-purpose, claude) and future custom agents run unblocked. The hook has no model field, so the agent name prefix is the only signal.
- fix: in haiku-context-guard.py, `ALLOW_IO = ("haiku-",)` and `ALLOW_IO_EXACT = {"statusline-setup"}`. If agent_id is set and agent_type does not start with an ALLOW_IO prefix and is not in ALLOW_IO_EXACT, deny FILE_TOOLS, Bash and PowerShell.
- deny message: "sonnet/opus/비-haiku 에이전트는 파일·셸 직접 접근 금지 (v0.8.2). haiku-scout(읽기)/haiku-batch-worker(쓰기)/haiku-coder(코드)에 위임하세요."
- agent-model-pin.py: verify it already rejects spawning haiku-* names with a model=sonnet|opus override. If not, add that rejection (otherwise a "haiku-scout" running opus bypasses the block).
H3 Bash/PowerShell: deny entirely for blocked agents, drop the regex for them:
- problem: regex READ/WRITE leaves bypasses (bash -c, sh x.sh, python3 script.py, go run, `< file`, xargs, eval, git log -p/blame/checkout/apply, curl -o, bin/chaude subcmds). It also gives false positives (any `>` in args, ls/wc/tail in pipes).
- fix: blocked agents get an unconditional deny on Bash/PowerShell. sonnet/opus agents have tools: Agent anyway, so this is belt-and-braces with zero bypass. Keep the existing regex only for the Haiku allow_large rule (unchanged, D6).
- removes: the PS_FILE alias false positives, and sample rows that test regex for sonnet (rewrite those rows as "any Bash -> deny").

## MED
M1 fail-closed for subagents:
- wrap the whole body, including `import _lean`, in try/except.
- on exception: if raw stdin contains `"agent_id"` and does not contain `"agent_type": "haiku-`, deny (exit 2 with the H2 message); otherwise exit 0.
- empty agent_type with agent_id set -> deny.
- main thread (no agent_id) stays fail-open (D7) so the session cannot brick itself.
M2 main session on sonnet/opus is not covered (user said "모든 sonnet/opus 모델"):
- today a forced `--model sonnet|opus` main is blocked only by main-thread-guard, which turns off in lean_off / 일반모드 / CLAUDE_ALLOW_MAIN_TOOLS=1.
- recommended fix: the chaude launcher exports `CHAUDE_MAIN_MODEL=<haiku|sonnet|opus>` (cmd/chaude, where --model is chosen). haiku-context-guard: if no agent_id and CHAUDE_MAIN_MODEL in {sonnet, opus}, deny FILE_TOOLS, Bash and PowerShell regardless of lean_off and CLAUDE_ALLOW_MAIN_TOOLS.
- needs: Go change, then 미정 #3 = yes. Confirm scope with the user (사용자 확인 #4 below).
M3 sonnet-reviewer with Agent tool reopens recursion (DECISIONS 35 assumed none):
- in agent-model-pin.py, allow callers sonnet-reviewer/sonnet-analyst/opus-architect to target only haiku-*. Deny sonnet-reviewer -> sonnet-reviewer.
- record this in notes/DECISIONS.md as an update to 35.
M4 writes of notes/runs by sonnet/opus: use haiku-batch-worker ("save verbatim, return path + wc -l"), not haiku-drafter. The drafter reads FACTS/DECISIONS and may paraphrase.
- update the plan's text for opus-architect.md, sonnet-analyst.md and CLAUDE.md line 12 accordingly. Keep haiku-drafter for prose that must be written in the user's voice.
M5 the reviewer loses charcount and git diff under option B:
- in sonnet-reviewer.md, replace "run bin/chaude charcount" with "haiku-checker runs `bin/chaude charcount <file>` and `git diff -- <paths>`, RETURN: verbatim <=4000 chars".
- the haiku-checker allow-list in settings is unchanged.

## LOW
L1 Pyright: apply the `getattr(_s, "reconfigure", None)` fix to every .claude/hooks/*.py that has `_s.reconfigure`, not only haiku-context-guard.py (main-thread-guard.py line 8 has it too).
- acceptance: `pyright .claude/hooks/` shows 0 errors and 0 warnings, and `grep -n "_s.reconfigure(" .claude/hooks/*.py` returns 0 hits.
L2 settings matcher change (DECISIONS 7): grep internal/ and cmd/ for "Read|Bash". Run `go test ./internal/settings/...` and `harness gen-settings windows` (or the equivalent dry-run) to confirm the Windows output picks up the new matcher.
L3 re-grep line numbers before editing (plan's are from a summary):
- sonnet-analyst: 11 answer path, 14 write-up, 18 scope, 23 plan review.
- opus-architect: step3 = 15, plan review = 23.
- sonnet-reviewer: tools 4, charcount 11, issue list 12.
L4 baseline: git status shows haiku-context-guard.py and settings.json already modified (M). The diff must be against the working tree. Run `git diff -- .claude/hooks/haiku-context-guard.py .claude/settings.json` first and save it to notes/runs/261030-verify-v082-block.md as "pre-state".
L5 grep Go sources, README and docs for "sonnet-analyst" + write/notes routing (delegateHint/planHint, TestHaikuPlanWorkflow) and main-thread-guard's deny text. Update any text that tells sonnet to write files directly.

## ordering (replace plan section 11)
1. Pre-state diff (L4). Live check: sonnet-analyst Read shows agent_type in the payload (before tool lists change, F12).
2. Step 0: Haiku return-channel rule (H1) and the acceptance call.
3. Hook rewrite (H2, H3, M1, L1 for this file) + agent-model-pin (H2, M3) + Pyright on all hooks (L1).
4. settings.json matcher + go test ./internal/settings (L2).
5. agents/*.md + CLAUDE.md text (M4, M5, L3, L5).
6. If M2 is approved: cmd/chaude env export + version 0.8.2 + go build/test.
7. CHANGELOG(s). Sample-input table re-run, with new rows:
- Explore Read -> deny.
- statusline-setup Edit -> allow.
- haiku-scout Read -> allow.
- sonnet-reviewer Bash `git diff` -> deny.
- bad JSON with agent_id -> deny.
- main + CHAUDE_MAIN_MODEL=opus + Read -> deny.
- main + CHAUDE_MAIN_MODEL=haiku + Read -> pass through to main-thread-guard.
8. sonnet-reviewer final review (using RETURN: verbatim). Stop. No commit/tag/push/release.

## 미정 recommendations
- 미정 #1 CHANGELOG 0.8.2 wording: "## 0.8.2\n- sonnet/opus(비-haiku) 서브에이전트의 파일·셸 직접 접근을 훅으로 전면 차단; 모든 읽기/쓰기는 haiku 에이전트 경유\n- haiku-context-guard 매처 확장: Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell\n- sonnet-analyst/sonnet-reviewer tools: Agent 전용\n- haiku 에이전트: `RETURN: verbatim` 지정 시 원문 발췌 반환 허용\n- 훅 Pyright 경고 수정(reconfigure getattr)"
- 미정 #2 docs/claude-lean/CHANGELOG.md: update only if it already has a 0.8.1 entry (grep "0.8.1"). If so, add a one-line 0.8.2 entry. Otherwise leave it untouched.
- 미정 #3 version bump cmd/chaude/main.go -> 0.8.2: yes if M2 is approved (Go code changes). Otherwise no, and record "harness-only change" in the CHANGELOG.
- 미정 #4 sonnet-analyst WebSearch/WebFetch: remove (tools: Agent only). Route web lookups to haiku-researcher with RETURN: verbatim. This matches "반드시 haiku를 경유" literally.

## 사용자 확인 recommendations
- 확인 #1 Pyright warning text: not required. Recommend applying L1 to all hooks and accepting "pyright .claude/hooks/ 0 warnings". Ask the user only if a warning is still visible in the IDE after that.
- 확인 #2 reviewer option: recommend B (Agent only), because A contradicts "읽기 전면 차단". B is conditional on H1 + M3 + M5; without them B degrades review to 300-char summaries.
- 확인 #3 lean_off keeps the block: recommend yes (D5). The user's instruction is unconditional. Escape hatch: env `CLAUDE_HARNESS_ALLOW_SONNET_IO=1`, documented in the CHANGELOG, off by default.
- 확인 #4 (new, user must answer): should a sonnet/opus MAIN session also be blocked (M2)? This needs a Go change and a version bump. Recommend yes, per "모든 sonnet/opus 모델".
