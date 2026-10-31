# qa: v0.8.2 plan summary (read-only, from 261030-explore-v082-plan-summary.md)
date: 2026-10-30
source: notes/runs/261030-explore-v082-plan-summary.md (70 lines, read fully)
caveat: quotes below are as the summary file quotes S3/S2; S3/S2 themselves not re-read here.

## a. confirmation F / G1 / statusline exception
- G1 (S3 "§10", line L177): main session (Opus). "live check: one sonnet-analyst Read; confirm PreToolUse payload has agent_type "sonnet-analyst""; accept: "agent_type value recorded in verify file. If absent: STOP, ask user (사용자 확인 필요 F)".
- F: "사용자 확인 필요 F" appears only inside G1 accept text. Summary §2 notes f-1 NOT FOUND (grep -i "f-1" = 0 in all 3 files).
- G1 in S2 (ordering step 1): "Live check: sonnet-analyst Read shows agent_type in the payload (before tool lists change, F12)." F12 undefined in these files.
- statusline exception = item A (S3 "§14", line L255, "사용자 확인 필요 A"): "statusline-setup exemption (review H2) not included: user words "haiku-* 외 전부". row 16 expects deny. if built-in /statusline must work, add ALLOW_IO_EXACT = {"statusline-setup"}."
- S2 H2 fix: ALLOW_IO = ("haiku-",), ALLOW_IO_EXACT = {"statusline-setup"}. S2 row: "statusline-setup Edit -> allow". S3 row 16 (L212): "deny (literal rule, 사용자 확인 필요 A)".
- status: open; not adopted in S3.

## b. rotate
- "rotate only": NOT FOUND.
- "rotate" only in S3 §6 status rows 19-20 (idle rotation timer, rotation/drain notes). Unrelated to G1 or A; fit not assessable.

## c. Bash handling
- "option 2" / "옵션 2" for Bash: NOT FOUND.
- reviewer options (S2 확인 #2): A keep Read/Grep/Glob, block write + Bash-write; B Agent only (chosen, S3 D2); C delete reviewer.
- adopted (S3 D1, §3.2 step 6; S2 H3): blocked agents: Bash and PowerShell denied entirely, no regex. "tool in FILE_TOOLS or tool in (Bash, PowerShell) -> deny(DENY_MSG)".
- removed: BASH_READ, BASH_WRITE, PS_FILE, _SEP (S3 §3.2 "removed:"). S2 H3 wanted regex kept for Haiku allow_large; S3 D6 removed it (allow_large reads tool_input.allow_large only).

## d. git-related items
- S0: "git diff -- .claude/hooks/haiku-context-guard.py .claude/settings.json" pre-state -> notes/runs/261030-verify-v082-block.md under "## pre-state".
- no commit, tag, push, release, gh release (S3 header L3, §10 L172, §13 L246).
- §12: "git status --short" -> no commit/tag; only §2 files + notes/runs/.
- sample row 6: sonnet-reviewer Bash "git diff" -> deny; S2 extra row same.
- reviewer git diff moved to haiku-checker, RETURN: verbatim <=4000 chars (S3 D2, §7.3; S2 M5).
- S11 write-routing grep covers internal/, cmd/, README.md, docs/.
- no Go source change; cmd/chaude/main.go version stays 0.8.1 (S3 D13).
- S6: go test ./internal/settings/... and go run ./cmd/chaude gen-settings windows.

## e. confirmation items (summary §5 doubts / §2)
- A: statusline-setup exemption. S2 recommends allow (H2 fix); S3 adopts deny (row 16). Recommended answer: not stated as such in files; conflict open.
- E (M2, 보류): main-session sonnet/opus not blocked.
- F: agent_type absent in G1 live check -> STOP, ask user.
- 미정 1: /usage popup model-call nature (status row 22).
- 미정 2: budget USD display location.
- S3 D2 reviewer option B chosen (Agent only); S2 options A/B/C.
- Recommended answers: none explicit in the summary file for E, F, 미정 1, 미정 2.

## doubts
- S3 §3.2: "from _lean import lean_off" outside try/except; step 3 says whole body in try/except (S2 M1 says include import). unresolved.
- S3 open_count refs stale (section 10 / section 9 vs list at §14).
- RETURN rule file count: S2 H1 = 5; S3 D8 = 8.
- S3 D14 "grep 0.8.1 docs/claude-lean/CHANGELOG.md = 0 hits": unverified.
- no verification run on S3 rules.

## not found
- "f-1", "rotate only", Bash "option 2": not found.
