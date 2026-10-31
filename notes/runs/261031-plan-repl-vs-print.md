# 261031 plan: claude -p vs REPL 포팅 vs PTY

## 사용자 요약 (Korean)
- 권고: (1) 현행 `claude -p --input-format stream-json` + `--resume` 유지·보강. (2) REPL Go 재구현은 하지 않음. (3) PTY 구동은 검증 전용 보조 도구로만 (주 경로 아님).
- 근거: 맥락 유지 문제는 `--resume <session-id>`로 이미 해결됨(실험 확인). -p 모드도 메인 스레드 캐시는 ephemeral_1h, /clear 후 prefix 캐시 재사용 확인 -> 포팅으로 얻을 성능 이득이 확인된 것이 없음.
- 포팅 비용: 도구 실행·권한·세션 저장·훅·CLAUDE.md·압축(compaction)·캐시 breakpoint·서브에이전트·MCP·구독 인증을 전부 다시 만들고, Claude Code 릴리스마다 따라가야 함. 구독(OAuth) 인증을 제3자 클라이언트에서 쓰는 것의 가능 여부·약관 적합성은 미확인.
- 미확인 항목: 11개 (아래 unverified 목록).

## goal
- decide: keep -p (stream-json) child vs port REPL to Go vs drive real REPL via PTY
- context problem: plain -p calls share no memory (confirmed); --resume <id> keeps context (confirmed). src: notes/runs/261031-claude-p-context-check.md
- usage: interactive prompt ~15k tokens larger; /usage is account total (cannot split by mode). src: notes/runs/261031-verify-via-go-results.md

## options
### opt1 keep -p + --resume (CHOSEN)
- pro: already implemented (core.go:145-153: -p, stream-json in/out, --verbose, --include-partial-messages, --model, --permission-prompt-tool stdio, --resume); structured events (session_id, usage, conversation_reset); /clear handled with prefix cache reuse (cache_read 11504); main thread 1h cache same as cli; Claude Code updates arrive for free
- con: ~15k token prompt difference vs interactive (cause per component unverified); hooks/settings merge in -p vs interactive not fully verified (item9); some interactive-only behavior (slash commands, TUI permission UX) not available
- fix needed: context continuity depends on harness persisting session_id and always passing --resume; --session-id pre-assign unverified
### opt2 port/reimplement REPL in Go (REJECTED)
- must reimplement: agent loop + streaming; tool defs + execution (Bash, Read, Edit, Write, Grep, Glob, WebFetch, WebSearch, Agent/subagents, TodoWrite, NotebookEdit, MCP client); permission rules (allow/deny/ask, settings precedence, prompt UX); settings.json merge (user/project/local/--settings); hooks (all events, matchers, JSON output semantics, merge order); CLAUDE.md + memory loading; session JSONL storage compatible with --resume; auto-compaction; prompt cache breakpoints + TTL selection; system prompt text; auth
- easy-to-miss: cache prefix must be byte-identical (system prompt + tool defs order/schema) or every turn pays cache_creation; tool schema drift per release; hook merge order and exit-code/JSON semantics; permission prompt for subagents; 1h vs 5m TTL choice for main vs subagent (observed main=1h, sub=5m); subagent context model; compaction summary format; session file format for interop
- blocker (미확인): subscription OAuth use by a third-party Go client — feasibility and terms unverified; API-key fallback changes billing to per-token (see FACTS.md price table)
- cost: large (multi-week), permanent maintenance tracking upstream; no confirmed benefit beyond the 15k prompt difference, which may be reducible inside opt1 (unverified)
### opt3 PTY-driven real REPL (VERIFY-ONLY)
- pro: exact interactive behavior (hooks/settings merge, interactive prompt size, permission UX); closes items 9, 7, 13
- con: output is ANSI TUI, no structured events -> fragile parsing; would need transcript JSONL tailing for structure; input timing/escape handling; breaks on UI changes; no creack/pty in repo yet
- use: separate probe command only, never main child path

## decision
- opt1 + bounded hardening; opt3 as optional probe; opt2 rejected unless opt1 hits a confirmed hard limit (record which limit in DECISIONS.md first)

## ordered steps
1. [haiku-runner] experiment --session-id (closes unverified U2)
   - cmd A: `U=$(uuidgen | tr A-Z a-z); claude -p --session-id "$U" "기억해 주세요: 과일 이름은 사과입니다. '네'라고만 답하세요."`
   - cmd B: `claude -p --resume "$U" "제가 말한 과일 이름은?"` -> expect 사과
   - cmd C (no resume): `claude -p --session-id "$U" "hi"` -> record whether error (id already used)
   - avoid sensitive words like "암호" (E5 refusal risk in prior check)
   - save: notes/runs/261031-session-id-check.md (key: value, result 확인/불가)
2. [haiku-runner] experiment --continue in isolated temp cwd (closes U1): mkdir fresh dir, two sessions, check --continue picks latest; save same file
3. [haiku-coder] only if step1 confirmed: internal/harness/core.go
   - add field `SessionID string` to the options struct used by NewSession
   - arg rule: if o.Resume != "" -> `--resume o.Resume` (unchanged); else if o.SessionID != "" -> `--session-id o.SessionID`; never both
   - caller: generate lowercase uuid v4 before spawn, store in registry row immediately (before first event), so a crash before the init event still leaves a resumable id
   - test: core_test or existing test file -> args contain --session-id when Resume empty; contain only --resume when both set
4. [haiku-coder] resume failure fallback
   - if child exits early after --resume with error (session not found), respawn without --resume and inject pending snapshot (existing handoff path)
   - must respect LESSONS 2026-10-29: no resume while draining; rotate must not overwrite a pending import snapshot
5. [haiku-scout -> sonnet-analyst] measure 15k prompt difference components (U5): compare first-request usage.input_tokens + cache_creation in transcripts for -p vs interactive on same cwd/settings; list differing parts (tool defs, system prompt, CLAUDE.md, hooks output). Analysis only, no code
6. [optional, haiku-coder] PTY probe: new `cmd/ptyprobe` (separate main, add github.com/creack/pty only to it), launches `claude` interactive, sends one prompt, exits; structure taken from transcript JSONL, not screen; goal close items 9/7/13. Do not import from internal/harness child path
7. [haiku-batch-worker] append DECISIONS.md line: `- [arch] 261031 user: "'claude -p' 방식 대신 claude code REPL 자체를 Go로 가져오는(재구현/포팅) 접근이 더 낫지 않은지" -> keep -p stream-json + --resume; REPL port rejected (reimplementation scope + OAuth unverified); PTY verify-only. Plan 261031-plan-repl-vs-print.`

## edge cases
- session file deleted/moved (cwd change) -> --resume fails -> step4 fallback
- --resume on a session still owned by a live child -> two writers to same JSONL (unverified behavior) -> never resume an id whose child is alive
- /clear in stream-json gives new session_id (conversation_reset) -> registry must update id on that event
- model switch (classifier.switchSession) parks id; resume must use same model family? (unverified U9)
- auto-compaction in long -p sessions: summary turn changes prefix -> cache_creation spike expected

## acceptance
- step1/2 files exist with 확인/불가 per flag
- `go test ./internal/harness/...` PASS after steps 3-4
- no change in args for existing Resume path (diff shows only additive lines)

## must NOT change
- base args at core.go:145-146 and --permission-prompt-tool stdio
- cache TTL / idle rotate 3599s logic (DECISIONS rotate 261028)
- hooks-via---settings decision (DECISIONS arch 261028, Plan 4-c)
- --bare not introduced (auth unverified)

## unverified (미확인, 11)
- U1 --continue isolated behavior
- U2 --session-id <uuid> (incl. reuse of same id)
- U3 --no-session-persistence
- U4 hook merge with --settings vs project settings in -p and interactive (item9)
- U5 component breakdown of ~15k interactive prompt difference
- U6 actual cache expiry timing (only request-level TTL observed)
- U7 items 7 and 13 full confirmation (need PTY)
- U8 subscription OAuth usable by third-party Go client (feasibility + terms)
- U9 --resume across model switch keeps context/cache
- U10 concurrent writers to same session JSONL
- U11 --bare auth with subscription

## [haiku] steps
- step1/2 haiku-runner, step3/4/6 haiku-coder, step5 haiku-scout then sonnet-analyst, step7 haiku-batch-worker
