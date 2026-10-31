# plan: v0.8.2 write-block - FINAL (original + opus review applied)
date: 2026-10-30
status: plan only. no code edited, no commit, tag, push, or release.
base: notes/runs/261030-plan-v082-write-block.md (original), notes/runs/261030-plan-v082-write-block-review.md (review, verdict FIX)
user_decision (verbatim): "b 방식 채택. 모든 sonnet/opus 모델은 읽기/쓰기 전면 차단, bash를 통한 읽기/쓰기도 차단. 반드시 haiku를 경유하도록 강제 구현."
user_addition: chaude status items must be computed in Go client without model call (section 6).
open_count: 미정 2 (new, no source text). original 미정 4 closed by review recommendation (section 9). 사용자 확인 필요 6 (section 10). 보류 1 (M2).

## 0. goal
- every subagent whose agent_type is not haiku-* has no direct file access.
- blocked tools: Read, Write, Edit, MultiEdit, NotebookEdit, Grep, Glob, Bash, PowerShell (all of Bash/PowerShell, no regex).
- every file touch goes through haiku-* agents.
- enforcement: PreToolUse hook (code), agent tools: lists (declared), CLAUDE.md (rule text).

## 1. decisions (final)
- D1 scope: block = agent_id set AND agent_type not startswith `haiku-` (allowlist, review H2). Bash and PowerShell denied entirely for blocked agents (review H3).
- D2 sonnet-reviewer: tools = Agent only (option B, 확인 #2 권장안). reads via haiku-scout with `RETURN: verbatim <=4000 chars`; charcount and git diff via haiku-checker with `RETURN: verbatim <=4000 chars` (M5).
- D3 sonnet-analyst: tools = Agent only. WebSearch/WebFetch removed; web lookups via haiku-researcher with `RETURN: verbatim <=4000 chars` (미정 #4 권장안).
- D4 opus-architect: tools already Agent. text changes only.
- D5 lean mode: block applies even when lean_off() is true (확인 #3 권장안). escape hatch: env `CLAUDE_HARNESS_ALLOW_SONNET_IO=1` skips the block for blocked agents; off by default; documented in CHANGELOG (확인 #3 권장안, see 사용자 확인 필요 B).
- D6 Haiku rules: allow_large deny kept (only Haiku rule). Regex is removed (no longer needed: the rule reads tool_input.allow_large only). analyst/reviewer exemption removed (dead code).
- D7 main thread (no agent_id): fail-open, exit 0. main-thread-guard.py handles it. M2 (main sonnet/opus) 보류.
- D8 return channel (review H1, extended): ALL eight haiku-* agent files (scout, checker, drafter, batch-worker, runner, researcher, extractor, coder) get the RETURN rule. review listed 5; researcher, extractor, coder added so any haiku agent can answer a verbatim brief.
- D9 fail-closed (review M1): hook body wrapped in try/except; see section 3.2.
- D10 notes writes by sonnet/opus (review M4): haiku-batch-worker ("save verbatim, return path + wc -l"). haiku-drafter only for prose in the user's voice.
- D11 agent-model-pin (review M3): subagent callers (sonnet-reviewer, sonnet-analyst, opus-architect) may target only haiku-*. the check must run BEFORE the lean_off() early exit, or it is skipped in default mode (plan D5).
- D12 Pyright (review L1, 확인 #1 권장안): getattr fix in ALL 8 hooks that have `_s.reconfigure(` (not only haiku-context-guard).
- D13 version: cmd/chaude/main.go stays 0.8.1 (M2 보류 => no Go change). CHANGELOG records "harness-only change".
- D14 docs/claude-lean/CHANGELOG.md: untouched (grep "0.8.1" = 0 hits, 미정 #2 rule).

## 2. file list (order = execution order, section 11)
1. .claude/hooks/haiku-context-guard.py  (rewrite; section 3)
2. .claude/hooks/agent-model-pin.py      (M3 + reconfigure; section 4)
3. .claude/hooks/{context-alert,correction-guard,main-thread-guard,ledger,session-context,style-adapter}.py (reconfigure line only)
4. .claude/settings.json                 (matcher line only)
5. .claude/agents/haiku-{scout,checker,drafter,batch-worker,runner,researcher,extractor,coder}.md (RETURN rule line)
6. .claude/agents/sonnet-analyst.md, sonnet-reviewer.md, opus-architect.md
7. CLAUDE.md                              (lines 10-12)
8. notes/DECISIONS.md                     (append one line, update to [agents] rule)
9. CHANGELOG.md                           (new 0.8.2 section)
not changed: .claude/hooks/_lean.py, Go sources (internal/*, cmd/*: grep "Read|Bash" = 0 hits), docs/claude-lean/CHANGELOG.md, cmd/chaude/main.go, Haiku tool lists.

## 3. file 1: .claude/hooks/haiku-context-guard.py
### 3.1 before
- matcher `Read|Bash`; body with BASH_READ / BASH_WRITE / PS_FILE regex; sonnet-analyst Read branch; analyst/reviewer exemption; allow_large Haiku deny.
### 3.2 after (rules, in order)
1. stdin reconfigure via `getattr(_s, "reconfigure", None)` (Pyright fix).
2. `sys.dont_write_bytecode = True`.
3. whole body inside try/except (M1). on exception:
   - raw stdin text matches `"agent_id"` AND does not match `"agent_type"\s*:\s*"haiku-` -> exit 2, stderr = DENY_MSG.
   - else exit 0.
4. parse JSON. no agent_id -> exit 0 (D7).
5. agent_id set and agent_type empty -> deny (M1).
6. agent_id set and agent_type not startswith `haiku-` -> (lean_off irrelevant):
   - if env `CLAUDE_HARNESS_ALLOW_SONNET_IO` == "1" -> exit 0 (escape hatch, 사용자 확인 필요 B).
   - tool in FILE_TOOLS or tool in (Bash, PowerShell) -> deny(DENY_MSG).
   - else exit 0 (e.g. Agent tool).
7. agent_type startswith `haiku-`: if lean_off() -> exit 0. tool == Read and tin.allow_large truthy -> deny("allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first.").
8. else exit 0.

- FILE_TOOLS = ("Read", "Write", "Edit", "MultiEdit", "NotebookEdit", "Grep", "Glob")
- DENY_MSG (Korean, verbatim): "sonnet/opus/비-haiku 에이전트는 파일·셸 직접 접근 금지 (v0.8.2). haiku-scout(읽기)/haiku-batch-worker(쓰기)/haiku-coder(코드)에 위임하세요."
- ALLOW_IO = ("haiku-",)  (no ALLOW_IO_EXACT; see 사용자 확인 필요 A)
- deny output: same JSON as before (hookSpecificOutput permissionDecision deny), exit 0. exception path uses exit 2.
- removed: BASH_READ, BASH_WRITE, PS_FILE, _SEP, analyst/reviewer exemption.

### 3.3 Pyright
- fix: `getattr(_s, "reconfigure", None)`; call only if present; try/except kept.
- acceptance: `pyright .claude/hooks/` 0 errors 0 warnings (section 12).

## 4. file 2: .claude/hooks/agent-model-pin.py
- reconfigure: getattr fix (D12).
- new check placed BEFORE `if lean_off() or ...` (D11):
  - data.agent_id set AND caller agent_type in {sonnet-reviewer, sonnet-analyst, opus-architect} AND tool_input.subagent_type not startswith `haiku-` -> deny: "sonnet/opus 서브에이전트는 haiku-* 에이전트만 호출할 수 있습니다 (v0.8.2)."
  - sonnet-reviewer -> sonnet-reviewer is denied by this rule.
- existing rules unchanged: model strip for haiku-/sonnet-/opus- (already rejects haiku-* with model=sonnet|opus by deleting `model`: H2 check = no change needed); built-in BLOCKED; claude-code-guide pin.
- docstring: update line "Called from inside a subagent ... sonnet-reviewer" to "haiku-* only".

## 5. file 3-4: other hooks and settings
- .claude/hooks/{context-alert,correction-guard,main-thread-guard,ledger,session-context,style-adapter}.py: replace `_s.reconfigure(encoding="utf-8")` with getattr form. no other change. main-thread-guard.py is otherwise unchanged.
- .claude/settings.json line 70 (haiku-context-guard matcher):
  - before: `"matcher": "Read|Bash",`
  - after: `"matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell",`
  - all other entries, permissions, env: unchanged.
  - the agent-model-pin entry (`Agent|Task`) unchanged.

## 6. Go status items (user addition: no model call)
Rule: every status shown in chaude is computed in the Go client from in-memory state, child stream events (produced by the turn already running), or local files. Exception row flagged.
Source key: STREAM = child claude stream-json events of the current turn; MEM = Go in-memory; AGSTORE = agent store JSON in state dir (internal/harness/agentstore.go); PROC = extra claude CLI process.

| # | status item | where shown | Go computation | source | model call |
|---|---|---|---|---|---|
| 1 | model name | status line left | r.a.Model (launch config) | config | no |
| 2 | ctx N | status line | r.lastCtx = input + cache_read + cache_creation (core.go ctxTokens, repl.go usageLine) | STREAM usage of last result | no (reuses API usage of the turn) |
| 3 | ctx threshold | status line; /status | thresholdFor(model) (repl.go effectiveThreshold) | flag/config | no |
| 4 | 5h rate "HH:MM/HH:MM · N%" | status line | fmtRateStatus(rate_limit_info) | STREAM rate_limit_info (repl.go setRate) | no |
| 5 | rot N | status line; /status | r.rotations counter | MEM | no |
| 6 | mode (default/acceptEdits/plan/auto) | status line when not default | r.mode atomic (mode.go) | MEM (user key/command) | no |
| 7 | agents N running (bg N) | status line | tracker.counts() over running() (drain.go) | STREAM Agent tool events; AGSTORE on restore | no |
| 8 | state 대기 / 생각중 + input placeholder | input box placeholder; main row | m.busy flag | MEM (send/result events) | no |
| 9 | turn elapsed "Nm NNs" | main row | time.Since(turnStart) | MEM clock | no |
| 10 | last tool name | main row | m.lastTool from tool_use event (tui.go) | STREAM | no |
| 11 | main row 생각중-모델-메인(요약) | agent panel | statusText + first prompt line, trunc 60 | MEM (curPrompt) | no |
| 12 | agent row 상태-모델-이름(요약) | agent panel | agentStatusText; state = agentState(flags Failed/Interrupted/Done/Started) | STREAM Agent events; AGSTORE for restored | no |
| 13 | agent state dot color | agent panel | agentDot(state) | same as 12 | no |
| 14 | panel scroll marker "… a-b/c" | agent panel | rowsA window (panOff) | MEM | no |
| 15 | queued prompts "대기 …" / "… +N 대기" | above input box | m.queue (max 5) | MEM | no |
| 16 | "↓ 새 메시지 (End)" | transcript bottom | m.unseen flag | MEM (output while scrolled up) | no |
| 17 | permission hint "y allow · n deny" / key hints | status line right | m.perm != nil | STREAM permission request | no |
| 18 | /status line (model, ctx, threshold, rotations, session id) | transcript note | localStatus() | MEM + STREAM init (session id) | no |
| 19 | idle cache-cold notice and rotation timer | transcript note | idleDeadline(idleRotate, effTTL) vs r.lastAPI; TTL from usage.cache_creation (core.go observedTTL) | STREAM usage | no |
| 20 | rotation / drain notes (dim lines) | transcript | tuiWriter notes from rotate/drain code | MEM | no |
| 21 | mouse ON/OFF note | transcript | m.mouse toggle | MEM | no |
| 22 | /usage popup | popup | usageFetch runs `claude -p /usage --output-format json --no-session-persistence` (tui.go:703) | PROC (claude CLI) | YES, possibly: CLI process; whether it uses API tokens = 미정 1 |

- model-call status rows: 1 (row 22, unresolved). rows 1-21 need no model call.
- not a status row but a model call: category/route classifier `claude -p --model <haiku|sonnet>` per prompt (classifier.go:188). routing decision, not displayed status. out of scope of this section. 사용자 확인 필요 C.
- budget (budget.go, USD from usage x price table in FACTS.md): display location not verified -> 미정 2.
- acceptance for section 6: haiku-scout greps each source file named above and confirms the function exists (no net new Go code in v0.8.2).

## 7. files 5-7: agent text
### 7.1 haiku-*.md (all 8): add one line after the existing "Return" rule:
"If the brief contains `RETURN: verbatim <=N chars`, the final message is verbatim excerpts with path:line, up to N chars (max 6000); the 300-char cap does not apply."
### 7.2 sonnet-analyst.md (line refs re-grep before edit, L3)
| before | after |
|---|---|
| `tools: Write, Bash, WebSearch, WebFetch, Agent` | `tools: Agent` |
| answer path write | answer draft via `haiku-drafter` (user-voice prose) |
| full write-up `notes/runs/<YYMMDD>-analyst-<topic>.md` | via `haiku-batch-worker` (verbatim save, M4) |
| `Scope first: you have no Read/Grep/Glob/Edit.` | `Scope first: you have no Read/Write/Edit/Grep/Glob/Bash/WebSearch/WebFetch.` |
| file reads | every read via `haiku-scout` with `RETURN: verbatim <=4000 chars`; web via `haiku-researcher` with same RETURN |
| plan review | `haiku-scout` reads, `haiku-batch-worker` writes improvements |
### 7.3 sonnet-reviewer.md
| before | after |
|---|---|
| `tools: Read, Grep, Glob, Bash` | `tools: Agent` |
| (none) | `Read nothing directly: haiku-scout reads and returns excerpts with RETURN: verbatim <=4000 chars.` |
| `bin/chaude charcount` run by reviewer | `haiku-checker` runs `bin/chaude charcount <file>` and `git diff -- <paths>`, `RETURN: verbatim <=4000 chars` (M5) |
| issue list `notes/runs/<YYMMDD>-review-<topic>.md` | written via `haiku-batch-worker` (M4) |
### 7.4 opus-architect.md (tools unchanged: Agent)
| before | after |
|---|---|
| `No direct file access (v0.8.1):` | `No direct file access (v0.8.2):` |
| `reading/search -> haiku-scout (or sonnet-analyst for synthesis)` | `reading/search -> haiku-scout (RETURN: verbatim <=4000 chars)` |
| `plans/answers written to notes/runs/ -> haiku-drafter/sonnet-analyst` | `plans/answers written to notes/runs/ -> haiku-batch-worker` (M4) |
| plan step `write notes/runs/<YYMMDD>-plan-<topic>.md` | `have haiku-batch-worker save notes/runs/<YYMMDD>-plan-<topic>.md verbatim` |
| plan review `read it and write only concrete improvements` | `haiku-scout reads it (RETURN: verbatim <=4000 chars); haiku-batch-worker writes only concrete improvements` |

## 8. file 6: CLAUDE.md
- line 10 after: `- sonnet/opus 에이전트 파일 직접 접근 전면 금지 (Read·Write·Edit·Grep·Glob 및 Bash/PowerShell 포함, 훅 강제): 파일 읽기는 haiku-scout(RETURN: verbatim <=4000 chars), 쓰기는 haiku-batch-worker/haiku-coder, 웹은 haiku-researcher가 수행.`
  (plan 8.2 wording kept except sonnet-analyst 3-file batching rule moved into agent text; 3-file batching rule stays in sonnet-analyst.md as `유사 파일끼리 최대 3개 파일/묶음`.)
- line 11: remove `may call sonnet-reviewer for review (the only non-Haiku agent a subagent may call)`. replace with `hand work only to haiku-* agents`. (D11)
- line 12 after: remove `haiku-drafter`/`sonnet-analyst` as writers for notes/runs; state `haiku-batch-worker` (verbatim) and `haiku-drafter` (user-voice prose only). (M4)
- lines 8-9 (routing: `Sonnet = 검토·분석`): unchanged.

## 9. file 7-8: CHANGELOG.md and DECISIONS.md
- CHANGELOG.md: insert above `## 0.8.1`:
```
## 0.8.2
- sonnet/opus(비-haiku) 서브에이전트의 파일·셸 직접 접근을 훅으로 전면 차단; 모든 읽기/쓰기는 haiku 에이전트 경유
- haiku-context-guard 매처 확장: Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell
- sonnet-analyst/sonnet-reviewer tools: Agent 전용
- sonnet/opus 서브에이전트는 haiku-* 에이전트만 호출 가능 (agent-model-pin)
- haiku 에이전트: `RETURN: verbatim <=N chars` 지정 시 원문 발췌 반환 허용 (max 6000)
- 훅 Pyright 경고 수정(reconfigure getattr, 모든 훅)
- 탈출구: 환경변수 CLAUDE_HARNESS_ALLOW_SONNET_IO=1 (기본 off)
- harness-only change (cmd/chaude version 0.8.1 유지)
```
- DECISIONS.md: append one line, format `- [agents] ...`: "v0.8.2 user: sonnet/opus/비-haiku 서브에이전트 파일·셸 전면 차단 (haiku-* 허용 목록); update to 35: sonnet-reviewer는 haiku-*만 호출 가능 (subagent caller); sonnet-reviewer/analyst/opus tools = Agent only." (verbatim user words in the first clause only.)

## 10. execution steps (Haiku units + main gates)
Haiku steps: 14 (S0-S13). main-session gates: 2 (G1, G2). Stop after G2. no commit, tag, push, release.

| step | run by | task | acceptance |
|---|---|---|---|
| S0 | haiku-scout | pre-state: `git diff -- .claude/hooks/haiku-context-guard.py .claude/settings.json` into `notes/runs/261030-verify-v082-block.md` under "## pre-state" | file has "## pre-state" and the diff text; final msg = path + line count |
| G1 | main session (Opus) | live check: one sonnet-analyst Read; confirm PreToolUse payload has agent_type "sonnet-analyst" | agent_type value recorded in verify file. If absent: STOP, ask user (사용자 확인 필요 F) |
| S1 | haiku-batch-worker | add RETURN rule (section 7.1) to haiku-{scout,checker,drafter,batch-worker,runner,researcher,extractor,coder}.md | `grep -l "RETURN: verbatim" .claude/agents/haiku-*.md` = 8 files; each contains "max 6000" |
| S2 | haiku-scout | acceptance call: brief = "Read notes/runs/261030-plan-v082-write-block.md section 3.2. RETURN: verbatim <=4000 chars." | final msg has >= 1000 chars of quoted lines with path:line (run after S1, new spawn) |
| S3 | haiku-coder | rewrite .claude/hooks/haiku-context-guard.py per section 3.2 (delete regex, allowlist, fail-closed, escape hatch, getattr) | `python3 -I -m py_compile` OK, then remove __pycache__; `grep -c "BASH_READ\|BASH_WRITE\|PS_FILE"` = 0; `grep -c "_s.reconfigure("` = 0; `grep -c "getattr(_s"` >= 1 |
| S4 | haiku-coder | agent-model-pin.py: section 4 (check before lean_off; reconfigure) | py_compile OK; `grep -c "_s.reconfigure("` = 0; the new check line index < lean_off line index |
| S5 | haiku-batch-worker | reconfigure getattr fix in 6 hooks (section 5) | `grep -n "_s.reconfigure(" .claude/hooks/*.py` = 0 hits; py_compile all hooks OK |
| S6 | haiku-coder | .claude/settings.json line 70 matcher (section 5); then `go test ./internal/settings/...` and `go run ./cmd/chaude gen-settings windows` | settings JSON valid; `grep -c "Read|Bash"` in settings.json = 0; new matcher present; go test PASS; gen-settings output contains new matcher |
| S7 | haiku-coder | sonnet-analyst.md, sonnet-reviewer.md, opus-architect.md (section 7). re-grep line numbers first (L3) | `grep -n "^tools:"` on those 3 = `tools: Agent` only; `grep -c "WebSearch\|WebFetch\|Bash"` in tools lines = 0; each file has "RETURN: verbatim <=4000 chars"; `grep -c "v0.8.1" opus-architect.md` = 0 |
| S8 | haiku-coder | CLAUDE.md lines 10-12 (section 8) | `grep -c "may call"` near sonnet-reviewer = 0; `grep -c "haiku-drafter\`/\`sonnet-analyst"` = 0 |
| S9 | haiku-batch-worker | append one line to notes/DECISIONS.md (section 9) | exactly one new line, format `- [agents] ...` |
| S10 | haiku-coder | CHANGELOG.md insert (section 9) above `## 0.8.1` | `head -3` shows `## 0.8.2` then bullets; "harness-only change" present; "CLAUDE_HARNESS_ALLOW_SONNET_IO=1" present |
| S11 | haiku-scout | L5 grep (read only): "sonnet-analyst", "sonnet-reviewer", "notes/runs" write routing in internal/, cmd/, README.md, docs/, .claude/hooks/main-thread-guard.py | hit list with path:line saved to verify file; no edits |
| S12 | haiku-coder | fix S11 hits that tell sonnet/opus to write files directly | re-run S11 grep: 0 write-routing hits; if 0 hits in S11, record "no change" and skip |
| S13 | haiku-scout | verification: section 11 sample rows, pin rows, pyright, self-check greps (section 12) | all rows match expected; pyright 0 errors 0 warnings; results saved to verify file |
| G2 | main session | final review by sonnet-reviewer via haiku-scout excerpts (RETURN: verbatim <=4000 chars) | issue list; no open HIGH. no commit/tag/push/release |

## 11. hook sample rows (expected after S3/S4)
method: JSON on stdin, run `python3 -I .claude/hooks/<file>.py` from repo root; expected = hook output (deny / allow).
| # | agent_id | agent_type | tool_name | tool_input | expected |
|---|---|---|---|---|---|
| 1 | set | sonnet-analyst | Read | file_path notes/FACTS.md | deny |
| 2 | set | sonnet-reviewer | Grep | pattern x | deny |
| 3 | set | opus-architect | Write | file_path notes/a.md | deny |
| 4 | set | sonnet-analyst | Edit | file_path a | deny |
| 5 | set | opus-architect | Glob | pattern * | deny |
| 6 | set | sonnet-reviewer | Bash | `git diff` | deny |
| 7 | set | sonnet-reviewer | Bash | `bin/chaude charcount notes/x.md --limit 10` | deny |
| 8 | set | opus-architect | Bash | `echo hi > notes/a.md` | deny |
| 9 | set | sonnet-analyst | Bash | `go test ./...` | deny |
| 10 | set | sonnet-analyst | Agent | prompt x | allow |
| 11 | set | haiku-scout | Read | file_path a, allow_large true | deny |
| 12 | set | haiku-scout | Read | file_path a | allow |
| 13 | set | sonnet-analyst | Bash | `echo x 2>/dev/null` | deny |
| 14 | set | sonnet-analyst | PowerShell | `Get-Content x` | deny |
| 15 | set | Explore | Read | file_path a | deny |
| 16 | set | statusline-setup | Edit | file_path a | deny (literal rule, 사용자 확인 필요 A) |
| 17 | set | haiku-scout | Edit | file_path a | allow |
| 18 | set | (raw text has "agent_id", JSON broken) | - | - | deny, exit 2 |
| 19 | (absent) | (absent) | Read | file_path a | allow, exit 0 (main thread) |
| 20 | set | (empty) | Read | file_path a | deny |
| 21 | set | sonnet-analyst | Read | file_path a, env CLAUDE_HARNESS_ALLOW_SONNET_IO=1 | allow |
| 22 | set | sonnet-analyst | Read | file_path a, lean_off true (default) | deny (D5) |
| 23 | set | sonnet-analyst | Agent | prompt x | allow |
pin rows (agent-model-pin.py, tool Agent):
| P1 | set | sonnet-analyst | subagent_type sonnet-reviewer | deny (M3) |
| P2 | set | sonnet-reviewer | subagent_type sonnet-reviewer | deny (M3) |
| P3 | set | sonnet-reviewer | subagent_type haiku-scout | allow |
| P4 | set | haiku-scout | subagent_type haiku-coder, model opus | allow, model removed |
| P5 | (absent) | main | subagent_type sonnet-reviewer | allow |
main + sonnet/opus forced model (M2): not tested here (보류).

## 12. self-check (haiku-scout, S13)
- `pyright .claude/hooks/` -> 0 errors, 0 warnings.
- `grep -n "_s.reconfigure(" .claude/hooks/*.py` -> 0 hits.
- `grep -n "Read|Bash" .claude/settings.json` -> 0 hits; `grep -n "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell" .claude/settings.json` -> 1 hit.
- `grep -n "^tools:" .claude/agents/sonnet-*.md .claude/agents/opus-*.md` -> `tools: Agent` only.
- `grep -l "RETURN: verbatim" .claude/agents/haiku-*.md` -> 8 files.
- `grep -n "BASH_READ\|BASH_WRITE\|PS_FILE" .claude/hooks/haiku-context-guard.py` -> 0 hits.
- `grep -n "may call" CLAUDE.md` -> no sonnet-reviewer call permission.
- `grep -n "0.8.1" docs/claude-lean/CHANGELOG.md` -> unchanged (untouched).
- `git status --short` -> no commit/tag; no new files beyond the list in section 2 and notes/runs/.

## 13. must NOT change
- lean mode file .claude/hooks/_lean.py; lean_off() semantics.
- main-thread-guard.py behavior (only line 8 Pyright getattr).
- Haiku tool lists (haiku-researcher keeps WebSearch/WebFetch).
- allow_large deny for Haiku.
- permission allow entries; env block; agent matcher `Agent|Task`.
- Go sources and cmd/chaude version (0.8.1).
- no commit, tag, push, release, or gh release.

## 14. 미정 and 사용자 확인 필요
미정 (no source text):
- 미정 1: /usage popup (status row 22) model-call nature: claude CLI process; no source states whether it uses API tokens. Go-only computation of row 22 not possible without it.
- 미정 2: budget USD status (budget.go) display location in chaude: not verified in tui code.
closed by review recommendation (not open): original 미정 #1 CHANGELOG wording (section 9), #2 docs/claude-lean/CHANGELOG (untouched), #3 version bump (none, harness-only), #4 WebSearch/WebFetch (removed; haiku-researcher).

사용자 확인 필요:
- A: statusline-setup exemption (review H2) not included: user words "haiku-* 외 전부". row 16 expects deny. if built-in /statusline must work, add ALLOW_IO_EXACT = {"statusline-setup"}.
- B: escape hatch CLAUDE_HARNESS_ALLOW_SONNET_IO=1 included per 확인 #3 권장안 (off by default). keep or remove.
- C: category/route classifier `claude -p --model` per prompt is a model call (not a status row). confirm it stays out of the no-model-call rule.
- D: pyright installed locally: user-visible IDE warning text was not captured. acceptance = `pyright .claude/hooks/` 0/0 (확인 #1 권장안).
- E: 보류 (확인 #4): main session sonnet/opus block (M2, needs Go change CHAUDE_MAIN_MODEL + version bump). excluded from v0.8.2 per instruction.
- F: G1 live agent_type check unverified until run. if agent_type absent, plan stops.
