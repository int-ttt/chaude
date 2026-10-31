# scope: prompter loop (ESCALATE)
status: design confirmed, NOT implemented; policy change needs user decision
evidence:
- CLAUDE.md:5,17: Flow = one delegation round per prompt, one final reply, no interim text, no second round -> loop conflicts (A and B differ in how)
- haiku-prompter: not present anywhere (.claude, internal, CLAUDE.md, DECISIONS, FACTS grep 0); agents dir has 11 files, none prompter
- agent-model-pin.py:33,41: sonnet/opus callers -> haiku-* only; haiku callers -> haiku-* only (:41); no self-recursion guard; so prompter cannot call sonnet/opus and could call itself
- existing plan notes/runs/261030-plan-prompter-loop.md: options A (main re-asks, CLAUDE.md exception), B (loop inside haiku-prompter, recommended), C (none)
- memo notes/pastes/21b72660.md: user choice A/B/C still open; cap 3 re-asks
user rules to encode:
- haiku models excluded from recursion/loop handling
- no count/time given -> default 3 rounds; count or time given -> follow it
- time given -> start next job immediately when each job ends (until the deadline)
- prompter prints sub-agent / other-model results to TUI
design (B+, if approved):
1. new .claude/agents/haiku-prompter.md (tools Agent, Read): derive done-criteria from user verbatim; per round one Agent call, re-ask quoting unmet items; stop on criteria met / N rounds / deadline; round log + final answer in handback; never call self; N parse: explicit count > explicit time(deadline loop, next round immediately after previous ends) > default 3; skip loop if target agent is haiku-* work only? (open: user says haiku excluded = tasks the main/haiku itself would handle; confirm meaning)
2. CLAUDE.md: add one line routing 반복/재귀/계속/될 때까지/loop/until requests to haiku-prompter once per round; main prints handback in full; Flow round count unchanged
3. agent-model-pin.py: deny prompter->prompter; optional allow prompter->sonnet-analyst/sonnet-reviewer (conflicts with D11)
4. verify: gofmt/go vet/go test (Go untouched unless TUI live view needed)
conflicts / limits:
- live per-round TUI output impossible inside one foreground round under B; only handback printed after it ends (agent panel shows nested progress). Real-time per-round print needs A (main re-asks) = Flow exception
- time-bounded loop (e.g. until 5:20) inside one foreground subagent may hit timeouts; A or external scheduler more robust
- "haiku 제외" ambiguous: (i) haiku models never loop-targets, (ii) haiku main thread never loops itself
open decisions for user:
1. choose A / B / C (recommend B for Flow compatibility; A if live per-round TUI output required)
2. allow agent-model-pin.py edit (self-recursion guard)
3. allow prompter -> sonnet/opus (needs D11 exception)
4. meaning of "haiku 제외"
