# plan: main-opus (Part A)
status: GATED - apply only after user confirms (conflicts with DECISIONS.md:47 2026-10-31 "haiku main takes user input directly ... settings.json = haiku")
user_verbatim: "추천대로 진행하고, ..." (recommendation text not in notes; relayed by Haiku main as: default main -> Opus; Haiku only relays input and translates output)
goal: default main model = opus; Haiku role = input relay + output translation (haiku-tui) only; haiku-쓰기용 = haiku-batch-worker / haiku-coder; haiku-읽기용 = haiku-scout / haiku-researcher; no new agents
why_gated: CLAUDE.md and .claude/settings.json are config; change needs the user's own confirmation, not an agent relay

## steps (haiku-coder, in order)
1. internal/harness/repl.go:1186 `fs.StringVar(&a.Model, "model", "haiku", "")` -> "opus"
2. internal/harness/core.go:139-140 `if o.Model == "" { o.Model = "haiku" }` -> "opus"
3. internal/harness/repl.go:41 forwardRule ("You are the Haiku main ...") : inject only when a.Model == "haiku"; grep its use site first; with opus main inject nothing new
4. keep: orch.go:23-31 (e -> haiku, b/c/d/f -> opus), resident.go:28-29 fixed=opus, resident.go:42-43 min=haiku, `--model haiku` still works
5. .claude/settings.json:2 "model": "haiku" -> "opus"
6. CLAUDE.md:1,7,10 rewrite to: main = Opus (default); Haiku main only when `--model haiku`, then it relays input to opus-architect and haiku-tui translates output; write agents = haiku-batch-worker/haiku-coder (haiku-쓰기용), read agents = haiku-scout/haiku-researcher (haiku-읽기용)
7. plan/caveman-structure.md: add the same role mapping line (haiku-쓰기용 / haiku-읽기용) where agent roles are listed
8. notes/DECISIONS.md append: `[model-policy] 2026-11-01 user "추천대로 진행": default main = opus; supersedes 2026-10-31 entry (line 47)`

## acceptance
- go build ./... && go test ./internal/harness -count=1 PASS
- grep -n '"haiku"' repl.go core.go -> no default-model hits left (orch/resident excepted)
- grep "Haiku main" CLAUDE.md -> only in the --model haiku clause
## must NOT change
- agent definitions in .claude/agents/, orch modes, resident presets, model pin rules
