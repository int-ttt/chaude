# 261030 review: live sessions plan (self-review, draft v1 -> final)
plan: notes/runs/261030-plan-live-sessions.md (final already includes R1-R10)
tags: [fact] = seen in haiku-scout grep 261030; [inferred] = not verified in code

R1 key needs slot, not only role
- v1: key=cwd|model|role.
- issue: 5 coder sessions share role=coder -> collide in childSet. [fact: children.go:16 single key string; childSet children.go:70]
- change: key=cwd|model|role|slot (D1).

R2 "상한 제거" conflict likely false
- v1: treated DECISIONS:16 as removing a session-count cap, so new 8-slot cap superseded it.
- issue: DECISIONS:16 is tagged [rotate] and its text is "ctx >= 270000 -> handoff" -> context-size cap. [fact: tag and excerpt] [inferred: whole line not read]
- change: keep 16/46 valid; add separate [sessions] decision; supersede 16 only if full text says session count (D5).

R3 prompter: per-prompt relaunch -> subagent per request + relaunch at expiry
- v1: (B) /exit + relaunch after every prompt output.
- issue: haiku-prompter subagent already starts with fresh context each call; per-prompt relaunch adds process start + cache write each time. cross-process cache hit within TTL [inferred].
- change: (C) in D4; (B) kept as fallback if leftover context is measured to affect answers.

R4 parked map keyed by model only
- v1: missed.
- issue: repl.go:175 parked map model -> session id, classifier.go:258-285 parked[model]. three haiku roles would overwrite each other's parked id. [fact]
- change: step 4, key parked by full childKey; only RoleMain uses Haiku-switch park.

R5 parked digest can leak tui/prompt output into opus
- v1: missed.
- issue: livechild.go:171 injects "## While this session was parked" on resume [fact]; whether it includes other roles' output [inferred, digest source not read].
- change: step 5 role filter; acceptance TestParkedDigestRole.

R6 coder cap reached -> queue, not refuse
- v1: reused existing refuse path.
- issue: dispatch.go:43 "워커 세션 상한(%d개)에 도달해 새 세션을 시작하지 않습니다" drops work [fact]; with bundling, work should wait.
- change: actQueue + coderPool FIFO (D2, D3).

R7 parallel coders editing the same file
- v1: bundling by Kind only.
- issue: two coders on one file -> lost edits [inferred risk].
- change: per-file lease is rule 1, ahead of Kind (D3).

R8 standby slots lazy, not pre-warmed
- v1: pre-spawn 4 standby coders.
- issue: idle sessions expire at wsMaxIdle 270s [fact: orchws.go:34]; keeping them warm needs repeated pings/cache refresh [inferred cost].
- change: lazy spawn; user question left open in plan.

R9 -max-workers default 3 contradicts 5 coders
- v1: left DefaultMaxWorkers=3.
- issue: dispatch.go:12 DefaultMaxWorkers=3 [fact]; reviewer says unused in production [inferred, call sites not read].
- change: DefaultCoderCap=5, flag = coder cap, range 1..5 so user config 8 cannot be exceeded.

R10 replacement must not reuse /clear path
- v1: "reset session" unspecified.
- issue: /clear path calls endAllParked("clear") [fact: repl.go:1168]; user rule: no /clear.
- change: separate rotateRole (/exit, 10s, kill, respawn); TestRotateRole asserts no clear path.

## remaining risks / unverified
- pickChild call sites and whether limit is passed (claim "DefaultMaxWorkers unused" unverified).
- children launched as interactive `claude` or `claude -p` stream-json: if -p, "/exit" = close stdin and wait exit; rotateRole must handle both [inferred].
- coder ctx measurement source for the 24000 bundle limit (orchws st.Ctx assumed reusable) [inferred].
- cost numbers for session start vs bundling not measured; D3 thresholds (5 tasks, 24000) are starting values, tune from notes/metrics.
- full text of DECISIONS:16 and notes/research/prompt-cache-ttl-clear.md not read.
