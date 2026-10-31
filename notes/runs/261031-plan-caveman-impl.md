# plan: caveman 전체 구현 (S0~S5 + haiku-tui) + v0.9.0
status: ready
source_plan: plan/caveman-structure.md (v7, 정본; 3.1~3.6 형식, 6절 감시, 8절 단계, 14절 결정)
scope_in: notes/runs/261031-scope-caveman-impl.md
user: "계획된 항목 전부 구현; 테스트 실행 후 정상 동작 확인되면 버전 v0.9.0으로 올림"

## base / branch (U6=b, C8)
- new worktree: /Users/leekwangseok/PycharmProjects/claude-harness-impl, branch caveman-impl, branched from caveman-u8 (39db4ad = HEAD 529da82 + U8 done). cmd: git -C /Users/leekwangseok/PycharmProjects/claude-harness worktree add -b caveman-impl /Users/leekwangseok/PycharmProjects/claude-harness-impl caveman-u8
- why not master tree: U6=b decided; master has 39 uncommitted files (v0.8.2 work) -> must not be bundled
- do NOT touch: /Users/leekwangseok/PycharmProjects/claude-harness (master tree), /Users/leekwangseok/PycharmProjects/claude-harness-caveman (stale caveman-relay-impl, uncommitted, old design), branch caveman-u8 itself
- one commit per step (S0..S5, tui, version). no merge, no push.

## 보류 항목 해소 (구현 단계 조사 결과, plan 14절 "구현 단계에서 처리")
- U3: PostToolUse matcher = "Agent|Task" (same as existing PreToolUse .claude/settings.json:50). evidence: settings.json has no PostToolUse yet
- U4: mode f live child ON by default (internal/harness/repl.go:1090 default defaultClientsFile() = ~/.chaude/clients.json; empty only if --clients-file=""). plan 7절 :168 wording fix: "기본 실행에서 활성 (--clients-file=\"\"일 때만 꺼짐)". ws field stays required key, value none allowed when no ws
- U7: PostToolUse payload has tool_input.prompt + tool_response -> hook can see brief. rule: if tool_input.prompt has a line matching ^lang:\s*ko\b, return must contain summary_ko; else not checked. verify with a unit test using a sample payload
- record all three in plan/caveman-structure.md 14절: [보류] -> [결정됨(구현 조사): ...] with the evidence above

## steps (order fixed)
S0 docs/claude-lean/CAVEMAN_FORMAT.md new: copy plan 3.1~3.6 formats as canonical (B형식 브리프, R-read, R-write, R-decide, status 4종, truncated yes|no, claim "<경로>:<행> |", summary_ko rule). agent files reference this doc instead of duplicating long text
S1 haiku-scout.md, haiku-researcher.md: R-read template + O2 exception line (per plan 8절 S1). hook .claude/hooks/caveman-format.py new (plan 6절): CAVEMAN_GUARD=log|warn|off default log; log -> 1 jsonl row in notes/metrics/<YYYY-MM-DD>.jsonl {"ts","hook":"caveman-format","agent","missing":[...]}; warn -> additionalContext 1 line "필수 필드 누락: <키>, 재요청"; off -> nothing. ALWAYS exit 0, never block (C8); any exception -> exit 0 silently. register in .claude/settings.json under PostToolUse matcher "Agent|Task". target set in S1 = {haiku-scout, haiku-researcher}
S2 haiku-coder, haiku-batch-worker, haiku-runner, haiku-checker, haiku-drafter: R-write template
S3 opus-architect, sonnet-analyst, sonnet-reviewer: B형식 brief + R-decide
S4 hook target set -> all agents from S1~S3 + haiku-tui + haiku-prompter (only the target-set constant changes; one place)
S5 CLAUDE.md:4 범위 한정 (plan Q1 wording), remove O2 exception lines added in S1 (per plan 8절 S5)
TUI .claude/agents/haiku-tui.md new (U9=a): role = user-facing Korean 존댓말 output of the main session (U8 해석 확인됨); input = R-* returns; output Korean; frontmatter model haiku, tools minimal (Read only). haiku-prompter.md: keep U8 relay block from caveman-u8; replace "main until then" note with haiku-tui
VER only if all checks PASS: cmd/chaude/main.go:16 var version = "0.9.0"; CHANGELOG.md top "## 0.9.0" entry (Korean, 1 line per step S0~S5, TUI, U3/U4/U7 해소, hook log mode). base CHANGELOG at HEAD top is 0.8.1; master's uncommitted 0.8.2 entry will be reconciled at merge (user decision)

## edge cases
- hook input not JSON / non-Agent tool / missing subagent_type -> exit 0, no output
- tool_response may be string or list of {type,text} blocks -> join text
- agent not in target set -> skip
- metrics dir missing -> create; write failure -> ignore
- keep every existing hook line in settings.json unchanged; only append PostToolUse block

## acceptance checks (haiku-runner in worktree)
1. cd /Users/leekwangseok/PycharmProjects/claude-harness-impl && go build ./... && go test ./... -> report PASS/FAIL counts
2. python3 -I -m py_compile .claude/hooks/*.py
3. python3 -I .claude/hooks/caveman-format.py fed 4 sample payloads via stdin: (a) complete R-read -> no row (b) missing status -> 1 row (c) lang: ko brief w/o summary_ko -> row lists summary_ko (d) garbage stdin -> exit 0. each exit code 0
4. json.load(.claude/settings.json) ok; grep PostToolUse matcher "Agent|Task"
5. grep: every target agent file references docs/claude-lean/CAVEMAN_FORMAT.md
6. sonnet-reviewer reads git diff caveman-u8..caveman-impl before VER commit

## must NOT change
- internal/harness/*.go, cmd/ (except version line), notes/LEDGER.md, notes/metrics existing rows
- existing PreToolUse/UserPromptSubmit/SubagentStop/SessionStart hooks
- master working tree; no merge into master (needs user decision: master 미커밋 39 files)

## open (user)
- merge: master 미커밋 변경(39 files, v0.8.2)을 먼저 커밋 후 caveman-impl 병합할지
